source .env.testnet

ORACLE_CODE_ID="$1"

INIT="{\"admin\": \"$ADMIN\"}"
ADDR=$(seid tx wasm instantiate $ORACLE_CODE_ID "$INIT" \
    --from $KEYNAME --label "Llama Oracle" --chain-id=sei-chain --gas=10000000 --fees=10000000usei --broadcast-mode=block --admin=$ADMIN -y \
    | grep -A 1 -m 1 "key: _contract_address" \
    | sed -n 's/.*value: //p' \
    | xargs)

echo "$ADDR"

