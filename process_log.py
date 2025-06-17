import sys
import os

def process_log_file(filepath):
    """
    Reads a log file, processes each line to extract data after '=>',
    and overwrites the file with the processed data.
    """
    if not os.path.isfile(filepath):
        print(f"Error: File '{filepath}' not found or is not a regular file.")
        return

    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        print(f"Read {len(lines)} lines from '{filepath}'.")
    except IOError as e:
        print(f"Error reading file '{filepath}': {e}")
        return

    processed_lines = []
    for line in lines:
        if '=>' in line:
            parts = line.split('=>', 1)
            if len(parts) > 1:
                processed_lines.append(parts[1].strip())

    print(f"Found {len(processed_lines)} lines to process.")
    if processed_lines:
        print("--- Content to be written (preview) ---")
        for l in processed_lines[:5]:
            print(l)
        if len(processed_lines) > 5:
            print("...")
        print("--------------------------------------")

    try:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write('\n'.join(processed_lines))
            if processed_lines:
                f.write('\n')
        print(f"Successfully processed and overwritten '{filepath}'.")
    except IOError as e:
        print(f"Error writing to file '{filepath}': {e}")


if __name__ == '__main__':
    if len(sys.argv) != 2:
        print("Usage: python3 process_log.py <path_to_file>")
        sys.exit(1)

    target_file = sys.argv[1]
    process_log_file(target_file)
