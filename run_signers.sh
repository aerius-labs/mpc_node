#!/bin/bash

# Configuration
KEYS_DIR="./"
LOG_DIR="./logs"
BINARY_NAME="tss_network"

# Create necessary directories
mkdir -p "$LOG_DIR"
mkdir -p "$KEYS_DIR"

# Function to validate key file
validate_key_file() {
    local key_file=$1
    if [ ! -f "$key_file" ]; then
        echo "Error: Key file not found: $key_file"
        return 1
    fi
    if [ ! -r "$key_file" ]; then
        echo "Error: Key file not readable: $key_file"
        return 1
    fi
    return 0
}

# Function to start a signer
start_signer() {
    local signer_id=$1
    local key_file="$KEYS_DIR/signer${signer_id}.store"
    local log_file="$LOG_DIR/signer${signer_id}.log"
    local pid_file="$LOG_DIR/signer${signer_id}.pid"

    # Validate key file
    if ! validate_key_file "$key_file"; then
        return 1
    fi

    # Check if signer is already running
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo "Signer $signer_id is already running with PID: $pid"
            return 1
        else
            rm "$pid_file"
        fi
    fi

    echo "Starting signer $signer_id with key file: $key_file"
    
    # Start the signer with nohup
    RUST_LOG=info nohup cargo run --bin signer -- \
        --key-file "$key_file" \
        >> "$log_file" 2>&1 &

    # Save PID
    local pid=$!
    echo $pid > "$pid_file"
    echo "Signer $signer_id started with PID: $pid"
    echo "Log file: $log_file"
}

# Function to stop a signer
stop_signer() {
    local signer_id=$1
    local pid_file="$LOG_DIR/signer${signer_id}.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo "Stopping signer $signer_id (PID: $pid)"
            kill "$pid"
            rm "$pid_file"
        else
            echo "Signer $signer_id is not running"
            rm "$pid_file"
        fi
    else
        echo "No PID file found for signer $signer_id"
    fi
}

# Function to check status
check_status() {
    local signer_id=$1
    local pid_file="$LOG_DIR/signer${signer_id}.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo "Signer $signer_id is running (PID: $pid)"
            return 0
        else
            echo "Signer $signer_id is not running (stale PID file)"
            rm "$pid_file"
            return 1
        fi
    else
        echo "Signer $signer_id is not running"
        return 1
    fi
}

# Function to view logs
view_logs() {
    local signer_id=$1
    local log_file="$LOG_DIR/signer${signer_id}.log"

    if [ -f "$log_file" ]; then
        tail -f "$log_file"
    else
        echo "No log file found for signer $signer_id"
    fi
}

# Function to start all signers
start_all() {
    echo "Starting all signers..."
    for id in {1..3}; do
        start_signer $id
        sleep 2  # Small delay between starts
    done
}

# Function to stop all signers
stop_all() {
    echo "Stopping all signers..."
    for id in {1..3}; do
        stop_signer $id
    done
}

# Main command processing
case "$1" in
    start)
        if [ "$2" = "all" ]; then
            start_all
        elif [ -n "$2" ]; then
            start_signer "$2"
        else
            echo "Usage: $0 start {all|<signer_id>}"
        fi
        ;;
    stop)
        if [ "$2" = "all" ]; then
            stop_all
        elif [ -n "$2" ]; then
            stop_signer "$2"
        else
            echo "Usage: $0 stop {all|<signer_id>}"
        fi
        ;;
    restart)
        if [ -z "$2" ]; then
            echo "Usage: $0 restart <signer_id>"
            exit 1
        fi
        stop_signer "$2"
        sleep 2
        start_signer "$2"
        ;;
    status)
        if [ -z "$2" ]; then
            echo "Status of all signers:"
            for id in {1..3}; do
                check_status $id
            done
        else
            check_status "$2"
        fi
        ;;
    logs)
        if [ -z "$2" ]; then
            echo "Usage: $0 logs <signer_id>"
            exit 1
        fi
        view_logs "$2"
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status|logs} {all|<signer_id>}"
        exit 1
        ;;
esac

exit 0