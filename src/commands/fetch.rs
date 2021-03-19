use crate::extensions::CreateEmbedExt;
use anyhow::*;
use lazy_static::lazy_static;

use super::*;
use std::collections::HashMap;

const SETFETCH_USAGE: &'static str = indoc::indoc!("
    Run this: 
    `curl -s https://raw.githubusercontent.com/unixporn/trup/prod/fetcher.sh | sh`
    and follow the instructions. It's recommended that you download and read the script before running it, 
    as piping curl to sh isn't always the safest practice. (<https://blog.dijit.sh/don-t-pipe-curl-to-bash>) 

    **NOTE**: use `!setfetch update` to update individual values (including the image!) without overwriting everything.
    **NOTE**: If you're trying to manually change a value, it needs a newline after !setfetch (update).
    **NOTE**: !git, !dotfiles, and !desc are different commands"
);

/// Run without arguments to see instructions.
#[command("setfetch")]
#[usage("setfetch [update]")]
#[sub_commands(set_fetch_update)]
pub async fn set_fetch(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let lines = args.rest().lines().collect_vec();
    handle_set_fetch(ctx, msg, lines, false).await
}

#[command("update")]
#[usage("setfetch update")]
pub async fn set_fetch_update(ctx: &client::Context, msg: &Message, args: Args) -> CommandResult {
    let lines = args.rest().lines().collect_vec();
    handle_set_fetch(ctx, msg, lines, true).await
}

pub async fn handle_set_fetch(
    ctx: &client::Context,
    msg: &Message,
    lines: Vec<&str>,
    update: bool,
) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

    if lines.is_empty() && msg.attachments.is_empty() {
        msg.reply_embed(&ctx, |e| {
            e.title("Usage").description(SETFETCH_USAGE);
        })
        .await?;
        return Ok(());
    }

    let mut info = lines
        .into_iter()
        .filter_map(|line| {
            const DELIM: &str = ":";
            let pos = line.find(DELIM)?;
            let key = &line[..pos];
            let value = &line[pos + DELIM.len()..];

            Some((key, value))
        })
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(k, v)| !k.is_empty() && !v.is_empty())
        .collect::<HashMap<String, String>>();

    for (key, value) in info.iter_mut() {
        if !ALLOWED_KEYS.contains(&key.as_ref()) {
            abort_with!(UserErr::Other(format!("Illegal fetch field: {}", key)))
        }
        match key.as_str() {
            "Memory" => {
                *value = byte_unit::Byte::from_str(&value)
                    .user_error("Malformed value provided for Memory")?
                    .get_appropriate_unit(false)
                    .to_string();
            }
            "image" => {
                if !util::validate_url(&value) {
                    abort_with!(UserErr::other("Got malformed url for image"))
                }
            }
            _ => {}
        }
    }

    let image = msg
        .embeds
        .iter()
        .find_map(|embed| embed.image.clone())
        .map(|image| image.url)
        .or_else(|| {
            msg.attachments
                .iter()
                .find(|a| a.dimensions().is_some())
                .map(|a| a.url.to_string())
        });

    if let Some(image) = image {
        info.insert("image".to_string(), image);
    }

    if update {
        db.update_fetch(msg.author.id, info).await?;
        msg.reply_success(&ctx, "Successfully updated your fetch data!")
            .await?;
    } else {
        db.set_fetch(msg.author.id, info).await?;
        msg.reply_success(&ctx, "Successfully set your fetch data!")
            .await?;
    }

    Ok(())
}

/// Fetch a users system information.
#[command("fetch")]
#[usage("fetch [user] [field]")]
pub async fn fetch(ctx: &client::Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<Db>().unwrap().clone();

    let guild = msg.guild(&ctx).await.context("Failed to load guild")?;
    let mentioned_user_id = if let Ok(mentioned_user) = args.single_quoted::<String>() {
        disambiguate_user_mention(&ctx, &guild, msg, &mentioned_user)
            .await?
            .ok_or(UserErr::MentionedUserNotFound)?
    } else {
        msg.author.id
    };

    let desired_field = args.single_quoted::<String>().ok();

    let (profile, fetch_info) = tokio::try_join!(
        db.get_profile(mentioned_user_id),
        db.get_fetch(mentioned_user_id),
    )?;
    if fetch_info.is_none() && profile.is_none() {
        abort_with!(UserErr::other("This user has not set their fetch :/"))
    }

    // all data shown in fetch, including the profile values
    let mut all_data: HashMap<String, String> = fetch_info.map(|x| x.info).unwrap_or_default();
    if let Some(profile) = profile {
        all_data.extend(profile.into_values_map());
    }

    let member = guild.member(&ctx, mentioned_user_id).await?;
    let color = member.colour(&ctx).await;

    match desired_field {
        // Handle fetching a single field
        Some(desired_field) => {
            let value = all_data.get(&desired_field).cloned()
                .user_error("Failed to get that value. Maybe the user hasn't set it, or maybe the field does not exist?")?;

            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("{}'s {}", member.user.name, desired_field));
                e.color_opt(color);
                if desired_field == "image" {
                    e.image(value);
                } else {
                    e.description(value);
                }
            })
            .await?;
        }

        // Handle fetching all fields
        None => {
            msg.reply_embed(&ctx, |e| {
                e.author(|a| a.name(member.user.tag()).icon_url(member.user.face()));
                e.title(format!("Fetch {}", member.user.tag()));
                e.color_opt(color);

                if let Some(image_url) = all_data.get("Distro").and_then(|d| find_distro_image(d)) {
                    e.thumbnail(image_url);
                }

                // remove image here, such that it is not included in the main fetch fields
                if let Some(image_url) = all_data.remove("image") {
                    e.image(image_url);
                }

                // set main fetch fields
                for (key, value) in all_data {
                    e.field(key, value, true);
                }
            })
            .await?;
        }
    }

    Ok(())
}

fn find_distro_image(distro: &str) -> Option<&str> {
    DISTRO_IMAGES
        .iter()
        .find(|(d, _)| distro.to_lowercase().starts_with(*d))
        .map(|(_, url)| *url)
}

lazy_static! {
    pub static ref ALLOWED_KEYS: [&'static str; 14] = [
        "Distro",
        "Kernel",
        "Terminal",
        "Editor",
        "DE/WM",
        "Bar",
        "Resolution",
        "Display Protocol",
        "Shell",
        "GTK3 Theme",
        "GTK Icon Theme",
        "Cpu",
        "Gpu",
        "Memory",
    ];
    static ref DISTRO_IMAGES: Vec<(&'static str, &'static str)> = vec![
        ("nixos", "https://nixos.org/logo/nixos-hires.png"),
        ("android", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3b/Android_new_logo_2019.svg/320px-Android_new_logo_2019.svg.png"),
        ("antix", "https://antixlinux.com/wp-content/uploads/2017/03/logo_antiX.png"),
        ("arch", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/74/Arch_Linux_logo.svg/250px-Arch_Linux_logo.svg.png"),
        ("archbang", "https://upload.wikimedia.org/wikipedia/commons/2/2c/ArchBangLogo.png"),
        ("archlabs", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/73/Default_desktop.png/300px-Default_desktop.png"),
        ("artix", "https://artixlinux.org/img/artix-logo.png"),
        ("alpine", "https://upload.wikimedia.org/wikipedia/commons/thumb/e/e6/Alpine_Linux.svg/250px-Alpine_Linux.svg.png"),
        ("alt", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/44/AltLinux500Desktop.png/250px-AltLinux500Desktop.png"),
        ("antergos", "https://upload.wikimedia.org/wikipedia/en/thumb/9/93/Antergos_logo_github.png/150px-Antergos_logo_github.png"),
        ("backbox", "https://upload.wikimedia.org/wikipedia/commons/thumb/b/b2/BackBox_4.4_Screenshot.png/250px-BackBox_4.4_Screenshot.png"),
        ("boss", "https://upload.wikimedia.org/wikipedia/en/f/f2/Bharat_Operating_System_Solutions_logo%2C_Sept_2015.png"),
        ("bodhi", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/fc/Bodhi_Linux_Logo.png/250px-Bodhi_Linux_Logo.png"),
        ("calculate", "https://upload.wikimedia.org/wikipedia/commons/3/3a/CalculateLinux-transparent-90x52.png"),
        ("caos", "https://upload.wikimedia.org/wikipedia/en/4/4b/CAos_Linux_logo.png"),
        ("centos", "https://upload.wikimedia.org/wikipedia/commons/thumb/b/bf/Centos-logo-light.svg/300px-Centos-logo-light.svg.png"),
        ("cub", "https://upload.wikimedia.org/wikipedia/commons/d/d8/CubLinux100.png"),
        ("debian", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4a/Debian-OpenLogo.svg/100px-Debian-OpenLogo.svg.png"),
        ("deepin", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/f5/Deepin_logo.svg/60px-Deepin_logo.svg.png"),
        ("devuan", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/f4/Devuan-logo.svg/320px-Devuan-logo.svg.png"),
        ("elementary", "https://upload.wikimedia.org/wikipedia/commons/thumb/8/83/Elementary_OS_logo.svg/300px-Elementary_OS_logo.svg.png"),
        ("emmabuntüs", "https://upload.wikimedia.org/wikipedia/commons/thumb/9/95/Emmabuntus_DE3_En.png/150px-Emmabuntus_DE3_En.png"),
        ("endeavouros", "https://upload.wikimedia.org/wikipedia/commons/thumb/e/e7/Endeavouros_Logo.svg/211px-Endeavouros_Logo.svg.png"),
        ("engarde", "https://upload.wikimedia.org/wikipedia/en/7/74/Engarde_Logo.png"),
        ("euleros", "https://upload.wikimedia.org/wikipedia/commons/thumb/e/e1/Operating_system_placement.svg/24px-Operating_system_placement.svg.png"),
        ("fedora", "https://upload.wikimedia.org/wikipedia/commons/thumb/0/09/Fedora_logo_and_wordmark.svg/250px-Fedora_logo_and_wordmark.svg.png"),
        ("fermi", "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a5/Fermi_Linux_logo.svg/80px-Fermi_Linux_logo.svg.png"),
        ("finnix", "https://upload.wikimedia.org/wikipedia/commons/thumb/5/52/Finnix-72pt-72dpi.png/100px-Finnix-72pt-72dpi.png"),
        ("foresight", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/48/Foresight_Linux_Logo_2.png/200px-Foresight_Linux_Logo_2.png"),
        ("freebsd", "https://upload.wikimedia.org/wikipedia/en/thumb/d/df/Freebsd_logo.svg/320px-Freebsd_logo.svg.png"),
        ("frugalware", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3c/Frugalware_linux_logo.svg/250px-Frugalware_linux_logo.svg.png"),
        ("fuduntu", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/2e/Fuduntu-logo.png/100px-Fuduntu-logo.png"),
        ("geckolinux", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/35/Tux.svg/35px-Tux.svg.png"),
        ("gentoo", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/48/Gentoo_Linux_logo_matte.svg/100px-Gentoo_Linux_logo_matte.svg.png"),
        ("hyperbola", "https://www.hyperbola.info/img/devs/silhouette.png"),
        ("instantos", "https://media.githubusercontent.com/media/instantOS/instantLOGO/master/png/light.png"),
        ("kali", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/4b/Kali_Linux_2.0_wordmark.svg/131px-Kali_Linux_2.0_wordmark.svg.png"),
        ("kanotix", "https://upload.wikimedia.org/wikipedia/commons/thumb/c/c4/Kanotix-hellfire.png/300px-Kanotix-hellfire.png"),
        ("kaos", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/2c/KaOS_201603.png/300px-KaOS_201603.png"),
        ("kde neon", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/f7/Neon-logo.svg/100px-Neon-logo.svg.png"),
        ("kororā", "https://upload.wikimedia.org/wikipedia/commons/thumb/6/6e/Korora_logo.png/250px-Korora_logo.png"),
        ("kubuntu", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/76/Kubuntu_logo_and_wordmark.svg/250px-Kubuntu_logo_and_wordmark.svg.png"),
        ("kwort", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/49/2019-11-24-121414_1280x1024_scrot.png/300px-2019-11-24-121414_1280x1024_scrot.png"),
        ("linux lite", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/79/Linux_Lite_Simple_Fast_Free_logo.png/250px-Linux_Lite_Simple_Fast_Free_logo.png"),
        ("linux mint", "https://upload.wikimedia.org/wikipedia/commons/thumb/5/5c/Linux_Mint_Official_Logo.svg/250px-Linux_Mint_Official_Logo.svg.png"),
        ("lunar linux", "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a1/Lunar_Linux_logo.png/200px-Lunar_Linux_logo.png"),
        ("macos", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/21/MacOS_wordmark_%282017%29.svg/200px-MacOS_wordmark_%282017%29.svg.png"),
        ("mageia", "https://upload.wikimedia.org/wikipedia/commons/thumb/9/93/Mageia_logo.svg/250px-Mageia_logo.svg.png"),
        ("mandriva", "https://upload.wikimedia.org/wikipedia/en/thumb/3/34/Mandriva-Logo.svg/200px-Mandriva-Logo.svg.png"),
        ("manjaro", "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a5/Manjaro_logo_text.png/250px-Manjaro_logo_text.png"),
        ("simplymepis", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/fc/MEPIS_logo.svg/100px-MEPIS_logo.svg.png"),
        ("mx linux", "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d4/MX_Linux_logo.svg/100px-MX_Linux_logo.svg.png"),
        ("netbsd", "https://upload.wikimedia.org/wikipedia/en/thumb/5/5c/NetBSD.svg/307px-NetBSD.svg.png"),
        ("openbsd", "https://upload.wikimedia.org/wikipedia/en/thumb/8/83/OpenBSD_Logo_-_Cartoon_Puffy_with_textual_logo_below.svg/320px-OpenBSD_Logo_-_Cartoon_Puffy_with_textual_logo_below.svg.png"),
        ("openmandriva lx", "https://upload.wikimedia.org/wikipedia/commons/thumb/6/60/Oma-logo-22042013_300pp.png/70px-Oma-logo-22042013_300pp.png"),
        ("opensuse", "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d0/OpenSUSE_Logo.svg/128px-OpenSUSE_Logo.svg.png"),
        ("oracle", "https://upload.wikimedia.org/wikipedia/commons/thumb/5/50/Oracle_logo.svg/200px-Oracle_logo.svg.png"),
        ("parted magic", "https://upload.wikimedia.org/wikipedia/commons/thumb/1/11/Parted_Magic_2014_04_28.png/300px-Parted_Magic_2014_04_28.png"),
        ("pclinuxos", "https://upload.wikimedia.org/wikipedia/commons/thumb/8/89/PCLinuxOS_logo.svg/80px-PCLinuxOS_logo.svg.png"),
        ("pinguy os", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/7a/Pinguy-os-desktop-12-04.png/300px-Pinguy-os-desktop-12-04.png"),
        ("pop!_os", "https://upload.wikimedia.org/wikipedia/commons/thumb/0/02/Pop_OS-Logo-nobg.png/125px-Pop_OS-Logo-nobg.png"),
        ("qubes os", "https://upload.wikimedia.org/wikipedia/commons/thumb/6/61/Qubes_OS_Logo.svg/120px-Qubes_OS_Logo.svg.png"),
        ("raspberry pi os", "https://upload.wikimedia.org/wikipedia/en/thumb/c/cb/Raspberry_Pi_Logo.svg/188px-Raspberry_Pi_Logo.svg.png"),
        ("red flag", "https://upload.wikimedia.org/wikipedia/commons/thumb/6/6c/RedFlag_Linux-Logo.jpg/180px-RedFlag_Linux-Logo.jpg"),
        ("red hat enterprise", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/46/RHEL_8_Desktop.png/300px-RHEL_8_Desktop.png"),
        ("rosa linux", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/25/Logo_ROSA.jpg/250px-Logo_ROSA.jpg"),
        ("russian fedora remix project", "https://upload.wikimedia.org/wikipedia/commons/thumb/0/08/Rfremix_Logo9.png/300px-Rfremix_Logo9.png"),
        ("sabayon", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3d/Sabayon_5.4_logo.svg/70px-Sabayon_5.4_logo.svg.png"),
        ("sailfish os", "https://upload.wikimedia.org/wikipedia/en/thumb/d/d3/Sailfish_logo.svg/250px-Sailfish_logo.svg.png"),
        ("scientific", "https://upload.wikimedia.org/wikipedia/commons/thumb/b/b1/Scientific_Linux_logo_and_wordmark.svg/80px-Scientific_Linux_logo_and_wordmark.svg.png"),
        ("slackware", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/22/Slackware_logo_from_the_official_Slackware_site.svg/250px-Slackware_logo_from_the_official_Slackware_site.svg.png"),
        ("solus", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/ff/Solus.svg/100px-Solus.svg.png"),
        ("solydxk", "https://upload.wikimedia.org/wikipedia/en/d/df/SolydXK_logo_small.png"),
        ("sparkylinux", "https://upload.wikimedia.org/wikipedia/commons/thumb/1/16/SparkyLinux-logo-200px.png/110px-SparkyLinux-logo-200px.png"),
        ("suse linux enterprise desktop", "https://upload.wikimedia.org/wikipedia/commons/thumb/5/59/SLED_15_Default_Desktop.png/300px-SLED_15_Default_Desktop.png"),
        ("suse linux enterprise server", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/ff/SUSE_Linux_Enterprise_Server_11_installation_DVD_20100429.jpg/300px-SUSE_Linux_Enterprise_Server_11_installation_DVD_20100429.jpg"),
        ("tiny core linux", "https://upload.wikimedia.org/wikipedia/commons/0/02/Tcl_logo.png"),
        ("turbolinux", "https://upload.wikimedia.org/wikipedia/commons/thumb/f/f1/Turbolinux.png/250px-Turbolinux.png"),
        ("turnkey linux virtual appliance library", "https://upload.wikimedia.org/wikipedia/commons/thumb/a/a1/Image-webmin3.png/300px-Image-webmin3.png"),
        ("ubuntu budgie", "https://upload.wikimedia.org/wikipedia/commons/thumb/2/2e/UbuntuBudgie-Wordmark.svg/250px-UbuntuBudgie-Wordmark.svg.png"),
        ("ubuntu gnome", "https://upload.wikimedia.org/wikipedia/commons/thumb/4/41/Ubuntu_GNOME_logo.svg/250px-Ubuntu_GNOME_logo.svg.png"),
        ("ubuntu mate", "https://upload.wikimedia.org/wikipedia/commons/thumb/5/53/Ubuntu_MATE_logo.svg/250px-Ubuntu_MATE_logo.svg.png"),
        ("ubuntu", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Logo-ubuntu_no%28r%29-black_orange-hex.svg/250px-Logo-ubuntu_no%28r%29-black_orange-hex.svg.png"),
        ("univention corporate server", "https://upload.wikimedia.org/wikipedia/commons/thumb/b/b6/Univention_Corporate_Server_Logo.png/250px-Univention_Corporate_Server_Logo.png"),
        ("uruk", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/39/Logo_of_Uruk_Project.svg/250px-Logo_of_Uruk_Project.svg.png"),
        ("vine", "https://upload.wikimedia.org/wikipedia/commons/thumb/3/35/Tux.svg/35px-Tux.svg.png"),
        ("void", "https://upload.wikimedia.org/wikipedia/commons/thumb/0/02/Void_Linux_logo.svg/200px-Void_Linux_logo.svg.png"),
        ("whonix", "https://upload.wikimedia.org/wikipedia/commons/thumb/7/75/Whonix_Logo.png/200px-Whonix_Logo.png"),
        ("xubuntu", "https://upload.wikimedia.org/wikipedia/commons/thumb/b/b0/Xubuntu_logo_and_wordmark.svg/200px-Xubuntu_logo_and_wordmark.svg.png"),
        ("zorin", "https://upload.wikimedia.org/wikipedia/commons/thumb/1/14/Zorin_Logomark.svg/277px-Zorin_Logomark.svg.png"),
    ];
}
