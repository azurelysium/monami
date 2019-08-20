#!/bin/bash
DATA=$(./monami control --host 127.0.0.1 --port 12345 --secret `cat secret.txt` --function list-nodes)
sed '/\/\/ MONAMI-NODES-DATA/r'<(echo "var data = $DATA;") ./index.html.template > ./index.html
