#!/bin/bash -e

# This script runs a both brooklyn and sydney nodes (in --dev), then fetches metadata.
# The metadata is saved to metadata_ggx_brooklyn.scale and metadata_ggx_sydney.scale respectively.

cd $(dirname $0)/src/metadata

PORT=12349

function get_metadata() {
    attempts=0
    max_attempts=30
    while sleep 1; do
        (( attempts++ )) || true

        curl -sX POST -H "Content-Type: application/json" --data \
        '{"jsonrpc":"2.0","method":"state_getMetadata", "id": 1}' \
        localhost:$PORT | jq .result | cut -d '"' -f 2 | xxd -r -p > $1

        # Check if file is empty
        if [ ! -s "$1" ]; then
            echo "Fetched metadata $1 is empty... retrying (attempt $attempts/$max_attempts)"
            if [ $attempts -ge $max_attempts ]; then
                echo "Failed to fetch metadata $1"
                exit 1
            fi

            continue
        fi

        echo "SUCCESS"
        echo
        return
    done
}

function cleanup() {
    (docker rm -f $1 || true) 1>/dev/null 2>/dev/null
}

GGX_RS="../containers/ggx.rs"
# Use grep and cut to parse the strings into variables
DEFAULT_GGX_NODE_IMAGE=$(cat "$GGX_RS" | grep -E 'DEFAULT_GGX_IMAGE: &str' | grep -oE '".*?"' | sed -e 's/"//g')
DEFAULT_GGX_BROOKLYN_TAG=$(cat "$GGX_RS" | grep -E "GgxNodeNetwork::Brooklyn =>" | grep -oE '".*?"' | sed -e 's/"//g')
DEFAULT_GGX_SYDNEY_TAG=$(cat "$GGX_RS" | grep -E "GgxNodeNetwork::Sydney =>" | grep -oE '".*?"' | sed -e 's/"//g')

function start_node_get_metadata() {
    echo "Starting node $1 with image $DEFAULT_GGX_NODE_IMAGE:$2"
    cleanup $1
    docker run -dit --rm --name $1 -p $PORT:9944 \
        --entrypoint /usr/src/app/target/release/ggxchain-node \
        $DEFAULT_GGX_NODE_IMAGE:$2 \
        --dev --tmp --unsafe-rpc-external

    sleep 5

    get_metadata $1.scale
    echo "Metadata for $1 fetched and saved to $1.scale"
    cleanup $1
}

start_node_get_metadata "metadata_ggx_brooklyn" $DEFAULT_GGX_BROOKLYN_TAG
start_node_get_metadata "metadata_ggx_sydney" $DEFAULT_GGX_SYDNEY_TAG
