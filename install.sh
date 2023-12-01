#!/bin/bash

DEST_DIR="/home/leonardo/bin/videosmover/commander"
SERVICE_NAME="videosmover.service"

choose_dest_dir () {
    echo "Enter commander destination directory or leave blank to use default:"
    echo "Default: ${DEST_DIR}"

    read USER_INPUT

    if test "$USER_INPUT" 
    then
        DEST_DIR="$USER_INPUT"
    fi
}

choose_dest_dir

choose_service_name () {
    echo "Enter commander user service name or leave blank to use default:"
    echo "Default: ${SERVICE_NAME}"

    read USER_INPUT

    if test "$USER_INPUT" 
    then
        SERVICE_NAME="$USER_INPUT"
    fi
}

choose_service_name

echo ""
echo "Running tests"
cargo nextest run
if [ $? -ne 0 ]; then
  exit 1
fi

echo ""
echo "Building commander for release"
cargo build --release
if [ $? -ne 0 ]; then
  exit 1
fi

echo ""
echo "Stoppping running commander service"
systemctl --user stop ${SERVICE_NAME}
if [ $? -ne 0 ]; then
  exit 1
fi

echo "Installing commander to target destination"
cp target/release/commander "$DEST_DIR/commander"
if [ $? -ne 0 ]; then
  exit 1
fi

echo "Starting commander service"
systemctl --user start ${SERVICE_NAME}
if [ $? -ne 0 ]; then
  exit 1
fi

echo ""
echo "Done!"