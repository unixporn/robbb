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


[ "$token" ] && [ "$serverid" ] || exec \
	echo "Please add the bot's token and the server's ID to the script"

# clean the env
unset col start mod mute intern
unset showcase modcat feedback modlog botmod botlog botstuff humantrafficking

#
#  roles
#

# get general server info
# -> use jq to get the roles -> remove quotes -> reverse list
roles=$(curl -s -X GET \
	-H "Authorization: Bot $token" \
	-H "Content-Type: application/json" \
	"https://discord.com/api/v9/guilds/$serverid" \
	| jq '.roles[]' -c | tr -d '"' | tac)

# do a line-by-line loop of the roles to get the variables
while IFS=':,' read -r _ id _ name _; do
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

# get channel list -> use jq to put every channel on a new line -> delete quotes
channels=$(curl -s -X GET \
	-H "Authorization: Bot $token" \
	-H "Content-Type: application/json" \
	"https://discord.com/api/v9/guilds/$serverid/channels" | jq '.[]' -c | tr -d '"')

# do a line-by-line loop of the channels to get the variables
while IFS=':,' read -r _ a _ b _ c _ d _; do
	# discord is kinda weird with channels - this makes that easier to deal with
	# category: id, type, name
	# channel:  id, last_message, type, name
	set -- "$a" "$b" "$c" "$d"
	# id is always the first
	id=$1; shift
	# if second is null or a high number (likely snowflake)
	# assume that it's last_message and skip it
	[ "$1" = null ] || [ "$1" -gt 99 ] && shift
	name=$2

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
