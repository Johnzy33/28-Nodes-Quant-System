#!/bin/bash

# Define the pipe paths
DATA_PIPE="/tmp/mt5_data.pipe"
CMD_PIPE="/tmp/mt5_cmds.pipe"

# 1. Remove existing files if they exist to avoid 'File exists' errors
rm -f $DATA_PIPE $CMD_PIPE

# 2. Create the Named Pipes (FIFOs)
mkfifo $DATA_PIPE
mkfifo $CMD_PIPE

# 3. Set Permissions
# 666 allows both your Linux user and Wine's user to Read/Write
chmod 666 $DATA_PIPE
chmod 666 $CMD_PIPE

echo "🚀 Pipes created successfully:"
ls -l /tmp/mt5_*.pipe

echo "------------------------------------------------"
echo "MT5 Path (Wine): Z:\\tmp\\mt5_data.pipe"
echo "Rust Path:       /tmp/mt5_data.pipe"
echo "------------------------------------------------"