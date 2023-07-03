#!/bin/sh
# generate .env file for testing up's bot
#  https://github.com/unixporn/Supreme-Demolition-Droid

cat << EOF >&2
Note:  Don't forget to enable the [34m'Presence'[0m & [34m'Server members'[0m intents in the bot's settings
Link to template:  [34mhttps://discord.new/zkhTrUTEbtg9[0m
Link to add bot:   [34mhttps://discord.com/oauth2/authorize?scope=bot&client_id=[31m<BOT-SNOWFLAKE>[0m
EOF
# You can also export these variables in your normal environment
# If variable is empty, then ask the user to type (/paste) the new contents.
[ ! "$serverid" ] && printf "[input the template server's ID]: " >&2 && read -r serverid
[ ! "$token" ]    && printf "[input the bot's token]: "          >&2 && read -r token


# ${#VAR} == get length of variable
[ "${#token}" -ge 50 ] && [ "$serverid" -ge 999 ] || exec \
	echo "Please input/export a valid token & server ID" >&2

# clean the env
unset mod intern mute col start
unset announcements rules feedback showcase techsupport modcat botstuff modlog botmod botlog humantrafficking polls


#
#  roles
#

# curl general server info -> use jq to get the roles in an easily-parsable format
roles=$(curl -s -X GET \
	-H "Authorization: Bot $token" \
	-H "Content-Type: application/json" \
	"https://discord.com/api/v9/guilds/$serverid" \
	| jq -r '.roles | sort_by(.position) | reverse | .[] | [.id, .name] | join("\t")')

# do a line-by-line loop of the roles to get the variables
# note: posix sh doesn't support escape codes in variables
while IFS=$(printf '\t') read -r id name; do
	case $name in
		"@everyone")       ;;
		"mods")            mod=$id;;
		"unpaid intern")   intern=$id;;
		"mute")            mute=$id;;
		# colours
		"black")           col="$id" start=1;;
		*) [ "$start" ] && col="$col,$id";;
	esac
done << EOF
$roles
EOF


#
#  channels
#

# curl channel list -> use jq to change to an easily-parsable format
channels=$(curl -s -X GET \
	-H "Authorization: Bot $token" \
	-H "Content-Type: application/json" \
	"https://discord.com/api/v9/guilds/$serverid/channels" \
	| jq -r '.[] | [.id, .name] | join("\t")')

# do a line-by-line loop of the channels to get the variables
# note: posix sh doesn't support escape codes in variables
while IFS=$(printf '\t') read -r id name; do
	case $name in
		announcements)   announcements=$id;;
		rules)           rules=$id;;
		server-feedback) feedback=$id;;
		showcase)        showcase=$id;;
		tech-support)    techsupport=$id;;
		/root/)          modcat=$id;;
		polls)           polls=$id;;
		bot-stuff)       botstuff=$id;;
		mod-log)         modlog=$id;;
		bot-auto-mod)    botmod=$id;;
		bot-messages)    botlog=$id;;
		user-log)        humantrafficking=$id;;
	esac
done << EOF
$channels
EOF


cat << EOF

export DATABASE_URL=sqlite:base.db
export TOKEN=$token
export GUILD=$serverid
export ROLE_MOD=$mod
export ROLE_HELPER=$intern
export ROLE_MUTE=$mute
export ROLES_COLOR=$col
export CATEGORY_MOD_PRIVATE=$modcat
export CHANNEL_ANNOUNCEMENTS=$announcements
export CHANNEL_RULES=$rules
export CHANNEL_SHOWCASE=$showcase
export CHANNEL_FEEDBACK=$feedback
export CHANNEL_MODLOG=$modlog
export CHANNEL_AUTO_MOD=$botmod
export CHANNEL_BOT_MESSAGES=$botlog
export CHANNEL_MOD_BOT_STUFF=$botstuff
export CHANNEL_BOT_TRAFFIC=$humantrafficking
export CHANNEL_TECH_SUPPORT=$techsupport
export CHANNEL_MOD_POLLS=$polls
export CHANNEL_ATTACHMENT_DUMP=""
export ATTACHMENT_CACHE_PATH=./cache
export ATTACHMENT_CACHE_MAX_SIZE=50000000

EOF
