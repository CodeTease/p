p() {
    if [ "$1" = "j" ]; then
        if [ -z "$2" ]; then
            echo "Usage: p j <path>"
            return 1
        fi
        
        # Temp file to capture output
        local tmp_file=$(mktemp)
        
        # Run Pavidi with PAVIDI_OUTPUT env var
        PAVIDI_OUTPUT="$tmp_file" command p j "$2"
        
        # Check if pavidi succeeded
        if [ $? -eq 0 ]; then
            local target_dir=$(cat "$tmp_file")
            if [ -d "$target_dir" ]; then
                cd "$target_dir"
            fi
        fi
        
        rm -f "$tmp_file"
    else
        command p "$@"
    fi
}
