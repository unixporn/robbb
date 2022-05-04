pub use super::*;
use poise::{
    async_trait,
    serenity_prelude::{ApplicationCommandOptionType, CreateApplicationCommandOption},
    ApplicationCommandOrAutocompleteInteraction, SlashArgument,
};
#[allow(clippy::module_inception)]
use serde::{Deserialize, Serialize};
use serenity::client;
use std::{fmt, str::FromStr};

pub mod fetch;
pub use fetch::*;
pub mod setfetch;
pub use setfetch::*;

/// convert the field-value into the desired format.
/// Returns `None` if the string is empty, as empty values must not be included in embeds.
pub fn format_fetch_field_value(field_name: &FetchField, value: String) -> Option<String> {
    if !value.is_empty() {
        if *field_name == FetchField::Memory {
            if value == "0" {
                None
            } else {
                Some(format_bytes(&value))
            }
        } else {
            Some(value)
        }
    } else {
        None
    }
}

/// parse a string as a number of bytes, then format bytes as a human readable string.
fn format_bytes(s: &str) -> String {
    let as_num = s.parse::<u128>();
    match as_num {
        Ok(n) => byte_unit::Byte::from_bytes(n)
            .get_appropriate_unit(false)
            .to_string(),
        Err(_) => s.to_string(),
    }
}

fn find_distro_image(distro: &str) -> Option<&str> {
    DISTRO_IMAGES
        .iter()
        .find(|(d, _)| distro.to_lowercase().starts_with(d))
        .map(|(_, url)| *url)
}

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, Clone)]
pub enum FetchField {
    Distro,
    Kernel,
    Terminal,
    Editor,
    #[serde(rename = "DE/WM")]
    DEWM,
    Bar,
    Resolution,
    #[serde(rename = "Display Protocol")]
    DisplayProtocol,
    Shell,
    #[serde(rename = "GTK3 Theme")]
    GTK3,
    #[serde(rename = "GTK Icon Theme")]
    Icons,
    CPU,
    GPU,
    Memory,
    #[serde(rename = "image")]
    Image,
}

pub static FETCH_KEY_ORDER: [FetchField; 15] = [
    FetchField::Distro,
    FetchField::Kernel,
    FetchField::Terminal,
    FetchField::Editor,
    FetchField::DEWM,
    FetchField::Bar,
    FetchField::Resolution,
    FetchField::DisplayProtocol,
    FetchField::Shell,
    FetchField::GTK3,
    FetchField::Icons,
    FetchField::CPU,
    FetchField::GPU,
    FetchField::Memory,
    FetchField::Image,
];

impl fmt::Display for FetchField {
    fn fmt(&self, writer: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FetchField::DEWM => write!(writer, "DE/WM"),
            FetchField::DisplayProtocol => write!(writer, "Display Protocol"),
            FetchField::GTK3 => write!(writer, "GTK3 Theme"),
            FetchField::Icons => write!(writer, "GTK Icon Theme"),
            FetchField::Image => write!(writer, "Image"),
            _ => write!(writer, "{:?}", self),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Not a valid fetch field")]
pub struct FetchFieldParseError;

impl std::str::FromStr for FetchField {
    type Err = FetchFieldParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "distro" => Ok(Self::Distro),
            "kernel" => Ok(Self::Kernel),
            "terminal" => Ok(Self::Terminal),
            "editor" => Ok(Self::Editor),
            "dewm" | "de" | "wm" | "de/wm" => Ok(Self::DEWM),
            "bar" => Ok(Self::Bar),
            "resolution" => Ok(Self::Resolution),
            "display protocol" => Ok(Self::DisplayProtocol),
            "shell" => Ok(Self::Shell),
            "gtk theme" | "gtk3 theme" | "theme" | "gtk" => Ok(Self::GTK3),
            "icons" | "icon theme" | "gtk icon theme" => Ok(Self::Icons),
            "cpu" => Ok(Self::CPU),
            "gpu" => Ok(Self::GPU),
            "memory" => Ok(Self::Memory),
            "image" => Ok(Self::Image),
            _ => Err(FetchFieldParseError),
        }
    }
}

// TODORW this would be clean, but doing PopArgument is pain

#[async_trait]
impl SlashArgument for FetchField {
    async fn extract(
        _: &client::Context,
        _: ApplicationCommandOrAutocompleteInteraction<'_>,
        value: &serde_json::Value,
    ) -> Result<Self, poise::SlashArgError> {
        let s = value
            .as_str()
            .ok_or_else(|| poise::SlashArgError::CommandStructureMismatch("Expected String"))?;
        Ok(
            FetchField::from_str(s).map_err(|e| poise::SlashArgError::Parse {
                error: Box::new(e),
                input: s.to_string(),
            })?,
        )
    }
    fn create(builder: &mut CreateApplicationCommandOption) {
        builder.kind(ApplicationCommandOptionType::String);
        for value in FETCH_KEY_ORDER.iter() {
            builder.add_string_choice(value.to_string(), value.to_string());
        }
    }
}

pub static DISTRO_IMAGES: [(&str, &str); 90] = [
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
