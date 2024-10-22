contract="./artifacts/dca.wasm"

# This is an account with mnemonic:
# `kiwi valid tiger wish shop time exile client metal view spatial ahead`
#
# neutrond keys add demowallet1 --recover
account=demowallet1
chain_id=ntrntest
node=http://localhost:26657

neutrond config node $node
resp=$(neutrond tx wasm store $contract --from $account --chain-id $chain_id --gas-prices 0.025untrn --gas-adjustment 2.5 --gas auto --output json -y)
echo $resp
tx_hash=$(echo $resp | jq  -r ".txhash" )
echo "tx_hash: $tx_hash"
sleep 1

code_id=$(neutrond q tx $tx_hash --output json --node $node| jq -r '.events.[] | select(.type == "store_code") | .attributes.[] | select(.key == "code_id") | .value')
echo "code_id: $code_id"
resp=$(neutrond tx wasm instantiate $code_id '{"owner": "neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j", "denom_ntrn": "untrn", "denom_usd": "uibcusdc", "max_block_old": 20, "max_schedules": 20}' --label test-mmvault --admin neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j --gas auto --output json --chain-id $chain_id --from $account --gas-prices 0.125untrn --gas-adjustment 1.5 -y)
tx_hash=$(echo $resp | jq -r ".txhash")
sleep 1

contract_address=$(neutrond q tx $tx_hash --output json --node $node| jq -r '.events.[] | select(.type == "instantiate") | .attributes.[] | select(.key == "_contract_address") | .value')
echo "contract address: $contract_address"
# curl -s "http://localhost:26657/tx?hash=0x8E7FFBDF8DF1CDE8E79B24723E97569DA05CE66EF4AD822BAAE14BA1AF4E4D92&prove=true"

# Function to get current SLinky price
get_slinky_price() {
    echo "Current SLinky Price:"
    neutrond q wasm contract-state smart $contract_address '{"get_formated":{}}' --node $node --trace
    sleep 1
}

# Function to place liquidity
place_liquidity() {
    echo "Placing liquidity"
    neutrond tx dex place-limit-order neutron10h9stc5v6ntgeygf5xf945njqq5h32r54rf7kf "untrn" "uibcusdc" "[0]" 2000000 "GOOD_TIL_CANCELLED" --price 0.3 --from $account --chain-id $chain_id --node $node --fees 2000untrn -y > /dev/null 2>&1
    sleep 1
}

# Function to execute a contract command and print the result
execute_contract() {
    local action=$1
    local params=$2
    local amount=$3
    echo "Executing: $action"
    neutrond tx wasm execute $contract_address "$params" $amount --from $account --chain-id $chain_id --gas-prices 0.025untrn --gas-adjustment 2.5 --gas auto --yes > /dev/null 2>&1
    sleep 1
}

# Function to query contract state
query_contract() {
    local query=$1
    echo "Querying: $query"
    neutrond q wasm contract-state smart $contract_address "$query" --node $node --output json | jq '.'
    sleep 1
}

# Function to print a section header
print_header() {
    echo -e "\n==== $1 ===="
}

# Demo script
print_header "Current SLinky Price"
get_slinky_price
print_header "Placing DEX Liquidity"
place_liquidity

print_header "Creating Schedules"
execute_contract "Deposit DCA 1" '{"deposit_dca": {"max_sell_amount": "5000", "max_slippage_basis_points": 10}}' "--amount 10000uibcusdc"
execute_contract "Deposit DCA 2" '{"deposit_dca": {"max_sell_amount": "5000", "max_slippage_basis_points": 10}}' "--amount 15000uibcusdc"

print_header "Active Schedules"
query_contract '{"get_schedules":{"address": "neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j"}}'

print_header "Withdrawing All Schedules"
execute_contract "Withdraw All" '{"withdraw_all": {}}'

print_header "Active Schedules After Withdrawal"
query_contract '{"get_schedules":{"address": "neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j"}}'

print_header "Creating New Schedules"
execute_contract "Deposit DCA 3" '{"deposit_dca": {"max_sell_amount": "5000", "max_slippage_basis_points": 10}}' "--amount 10000uibcusdc"
execute_contract "Deposit DCA 4" '{"deposit_dca": {"max_sell_amount": "5000", "max_slippage_basis_points": 10}}' "--amount 15000uibcusdc"

print_header "New Active Schedules"
query_contract '{"get_schedules":{"address": "neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j"}}'

print_header "Running Schedules"
for i in {1..3}; do
    print_header "Run $i"
    execute_contract "Run Schedules" '{"run_schedules": {}}'
    query_contract '{"get_schedules":{"address": "neutron13nfu3ct5xkr0vlswgk3gl9zazp7zan88edz67j"}}'
done
