#!/bin/sh

source .env.testnet


echo "Building binaries..."
./devops/build.sh


code_oracle=$(seid tx wasm store artifacts/oracle.wasm \
    -y --from=$KEYNAME --chain-id=sei-chain --gas=10000000 --fees=10000000usei --broadcast-mode=block \
    | grep -A 1 "code_id" \
    | sed -n 's/.*value: "//p' \
    | sed -n 's/"//p')

echo "oracle code ID: $code_oracle"

code_finance=$(seid tx wasm store artifacts/llama_finance.wasm \
    -y --from=$KEYNAME --chain-id=sei-chain --gas=10000000 --fees=10000000usei --broadcast-mode=block \
    | grep -A 1 "code_id" \
    | sed -n 's/.*value: "//p' \
    | sed -n 's/"//p')

echo "finance code ID: $code_finance"

echo "Initializing smart contracts..."
oracle_address=$(./devops/init_oracle.sh $code_oracle)
finance_address=$(./devops/init_finance.sh $code_finance $oracle_address)

echo "oracle address: $oracle_address"
echo "finance address: $finance_address"


