source .env.testnet

FINANCE_CODE_ID="$1"
ORACLE_ADDRESS="$2"


INIT="{\"oracle\": \"$ORACLE_ADDRESS\", \"admin\": \"$ADMIN\"}"
ADDR=$(seid tx wasm instantiate $FINANCE_CODE_ID "$INIT" \
    --from $KEYNAME --label "Llama Finance" -y --admin=$ADMIN \
    --chain-id=sei-chain --gas=10000000 --fees=10000000usei --broadcast-mode=block \
    | grep -A 1 -m 1 "key: _contract_address" \
    | sed -n 's/.*value: //p' \
    | xargs)

echo "$ADDR"
