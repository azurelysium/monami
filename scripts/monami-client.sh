#!/bin/bash

HOST="127.0.0.1"
PORT="12345"
SECRET="minamo"
INTERVAL="10"
OUTPUT_LIMIT="1200"

HOSTNAME=`hostname`
UUID=`uuidgen`
TIMESTAMP=`date +%s`
TAG="$1"
COMMAND="$2"

function send_status {
    OUTPUT=$(echo "`$COMMAND`" | sed ':a;N;$!ba;s/\n/\\n/g' | sed $'s/[^[:print:]\t]//g' | tail -c $OUTPUT_LIMIT)
    read -r -d '' MONAMI_MESSAGE << EOM
{
  "message_type": "Status",
  "message_status": {
    "hostname": "$HOSTNAME",
    "uuid": "$UUID",
    "tag": "$TAG",
    "command": "$COMMAND",
    "output": "$OUTPUT",
    "timestamp": $TIMESTAMP
  },
  "message_control": null
}
EOM

    echo -- `date`
    echo $MONAMI_MESSAGE

    # we used openssl version 1.1.1
    ENCRYPTED=$(echo -n $MONAMI_MESSAGE | openssl aes-256-cbc -iv 0 -md sha512 -pass "pass:$SECRET" -nosalt -a -A)
    RESPONSE=$(echo -n $ENCRYPTED | nc -u -w 1 $HOST $PORT)
    DECRYPTED=$(echo -n $RESPONSE | openssl aes-256-cbc -iv 0 -md sha512 -pass "pass:$SECRET" -nosalt -a -A -d)
    echo $DECRYPTED
}

while true; do
    send_status
    sleep $INTERVAL
done
