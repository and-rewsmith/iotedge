#!/bin/bash

###############################################################################
# This script builds the project
###############################################################################

set -e

###############################################################################
# These are the packages this script will build.
###############################################################################
packages=(mqttd)

###############################################################################
# Define Environment Variables
###############################################################################
# Get directory of running script
DIR=$(cd "$(dirname "$0")" && pwd)

BUILD_REPOSITORY_LOCALPATH=${BUILD_REPOSITORY_LOCALPATH:-$DIR/../../..}
PROJECT_ROOT=${BUILD_REPOSITORY_LOCALPATH}/mqtt
SCRIPT_NAME=$(basename "$0")
CARGO="${CARGO_HOME:-"$HOME/.cargo"}/bin/cargo"
RELEASE=

###############################################################################
# Print usage information pertaining to this script and exit
###############################################################################
usage()
{
    echo "$SCRIPT_NAME [options]"
    echo ""
    echo "options"
    echo " -h, --help          Print this help and exit."
    echo " -r, --release       Release build? (flag, default: false)"
    exit 1;
}

###############################################################################
# Obtain and validate the options supported by this script
###############################################################################
process_args()
{

    for arg in "$@"
    do
        case "$arg" in
            "-h" | "--help" ) usage;;
            "-r" | "--release" ) RELEASE="true";;
            * ) usage;;
        esac
    done
}

process_args "$@"

# ld crashes in the VSTS CI's Linux amd64 job while trying to link iotedged
# with a generic exit code 1 and no indicative error message. It seems to
# work fine if we reduce the number of objects given to the linker,
# by disabling parallel codegen and incremental compile.
#
# We don't want to disable these for everyone else, so only do it in this script
# that the CI uses.
>> "$PROJECT_ROOT/Cargo.toml" cat <<-EOF

[profile.dev]
codegen-units = 1
incremental = false

[profile.test]
codegen-units = 1
incremental = false
EOF

PACKAGES=
for p in "${packages[@]}"
do
    PACKAGES="${PACKAGES} -p ${p}"
done

# TODO: cd into directory (avoiding manifest path) then cp the files (avoiding target dir)
echo "Release build: $RELEASE"

# cd "$PROJECT_ROOT/mqttd"
cd "$PROJECT_ROOT"
if [[ -z ${RELEASE} ]]; then
    echo "Building artifacts"
    cd "$PROJECT_ROOT" && $CARGO build ${PACKAGES}
else
    EDGE_HUB_ARTIFACTS_PATH="target/publish/Microsoft.Azure.Devices.Edge.Hub.Service"
    MQTT_MANIFEST_PATH="./mqttd/Cargo.toml"
    LOCAL_TARGET_DIR_PATH="${PROJECT_ROOT}/target"
    TARGET_DIR_PATH="${BUILD_REPOSITORY_LOCALPATH}/${EDGE_HUB_ARTIFACTS_PATH}/mqtt/mqttd"
    mkdir -p "${TARGET_DIR_PATH}"

    echo "Building artifacts to ${TARGET_DIR_PATH}"

    # Build for linux amd64
    BUILD_COMMAND="cross build ${PACKAGES} --release --manifest-path ${MQTT_MANIFEST_PATH}"
    echo "Building for linux amd64"
    echo "${BUILD_COMMAND}"
    eval "${BUILD_COMMAND}"
    OUTPUT_BINARY="${LOCAL_TARGET_DIR_PATH}/release/mqttd"
    strip "${OUTPUT_BINARY}"
    cp ${OUTPUT_BINARY} ${TARGET_DIR_PATH} # needed because target-dir won't work with cross

    # Build for linux arm32 and linux arm64
    TARGET_PLATFORMS=("armv7-unknown-linux-gnueabihf" "aarch64-unknown-linux-gnu")
    for platform in "${TARGET_PLATFORMS[@]}"
    do
        BUILD_COMMAND_WITH_PLATFORM="${BUILD_COMMAND} --target $platform"
        echo "Building for $platform"
        echo "${BUILD_COMMAND_WITH_PLATFORM}"
        eval "${BUILD_COMMAND_WITH_PLATFORM}"

        OUTPUT_BINARY="${LOCAL_TARGET_DIR_PATH}/${platform}/release/mqttd"
        strip ${OUTPUT_BINARY}
        mkdir ${TARGET_DIR_PATH}/$platform && cp ${OUTPUT_BINARY} ${TARGET_DIR_PATH}/$platform # needed because target-dir won't work with cross
    done
fi
