#!/bin/sh
# generate .env file for testing up's bot
#  https://github.com/unixporn/Supreme-Demolition-Droid
#
# server template link:
#  https://discord.new/zkhTrUTEbtg9
#
# link to add bot to server:
#  https://discord.com/oauth2/authorize?scope=bot&client_id=<BOTID>
# where <BOTID> is the bot's snowflake

[ "$token" ] || token=TOKENGOESHERE
serverid=SERVERIDGOESHERE


[ "${#token}" -ge 50 ] && [ "$serverid" -ge 999 ] || exec \
	echo "Please add the bot's token and the server's ID to the script"

# clean the env
unset col start mod mute intern
unset showcase modcat feedback modlog botmod botlog botstuff humantrafficking

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
		server-feedback) feedback=$id;;
		showcase)        showcase=$id;;
		/root/)          modcat=$id;;
		mod-log)         modlog=$id;;
		bot-stuff)       botstuff=$id;;
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
export CHANNEL_SHOWCASE=$showcase
export CHANNEL_FEEDBACK=$feedback
export CHANNEL_MODLOG=$modlog
export CHANNEL_AUTO_MOD=$botmod
export CHANNEL_BOT_MESSAGES=$botlog
export CHANNEL_MOD_BOT_STUFF=$botstuff
export CHANNEL_BOT_TRAFFIC=$humantrafficking
export ATTACHMENT_CACHE_PATH=./cache
export ATTACHMENT_CACHE_MAX_SIZE=50000000
EOF
