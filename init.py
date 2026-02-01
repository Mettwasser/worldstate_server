#!/usr/bin/env python3
import json
import os
import shutil
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

# Configuration for fetch_data
OWNER = "WFCD"
REPO = "warframe-worldstate-data"
BRANCH = "master"
TARGET_FOLDER = "data"

def download_file(file_info):
    remote_path = file_info['remote_path']
    raw_url = file_info['raw_url']
    
    parent_dir = os.path.dirname(remote_path)
    if parent_dir and not os.path.exists(parent_dir):
        os.makedirs(parent_dir, exist_ok=True)
        
    print(f"Downloading {remote_path}...")
    try:
        # Add User-Agent to avoid potential 403s on some raw endpoints
        req = urllib.request.Request(raw_url, headers={'User-Agent': 'Python-Worldstate-Parser'})
        with urllib.request.urlopen(req) as response, open(remote_path, 'wb') as out_file:
            out_file.write(response.read())
    except Exception as e:
        print(f"Failed to download {remote_path}: {e}")

def fetch_data():
    api_url = f"https://api.github.com/repos/{OWNER}/{REPO}/git/trees/{BRANCH}?recursive=1"
    
    print(f"Fetching file list from {REPO}...")
    
    headers = {'User-Agent': 'Python-Worldstate-Parser', 'Accept': 'application/vnd.github+json'}
    req = urllib.request.Request(api_url, headers=headers)
    
    try:
        with urllib.request.urlopen(req) as response:
            tree_data = json.loads(response.read().decode())
    except Exception as e:
        print(f"Failed to fetch tree: {e}")
        return

    files_to_download = []
    
    for row in tree_data.get('tree', []):
        path_str = row['path']
        path_obj = Path(path_str)
        
        # Check if parent matches TARGET_FOLDER exactly (direct children)
        if str(path_obj.parent) == TARGET_FOLDER and path_obj.suffix == ".json" and row['type'] == "blob":
            raw_url = f"https://raw.githubusercontent.com/{OWNER}/{REPO}/{BRANCH}/{path_str}"
            files_to_download.append({
                'remote_path': path_str,
                'raw_url': raw_url
            })
            
    print(f"Found {len(files_to_download)} JSON files. Starting download...")
    
    # Use ThreadPoolExecutor to download in parallel
    with ThreadPoolExecutor(max_workers=10) as executor:
        executor.map(download_file, files_to_download)
        
    print("Data download complete.")

def fetch_drops():
    url = "https://drops.warframestat.us/data/all.json"
    output_dir = "drops"
    output_file = os.path.join(output_dir, "data.json")
    
    if not os.path.exists(output_dir):
        os.makedirs(output_dir)
        
    print(f"Downloading {url} to {output_file}...")
    
    headers = {'User-Agent': 'Python-Worldstate-Parser'}
    req = urllib.request.Request(url, headers=headers)
    
    try:
        with urllib.request.urlopen(req) as response, open(output_file, 'wb') as out_file:
            shutil.copyfileobj(response, out_file)
        print("Drops download complete.")
    except Exception as e:
        print(f"Failed to download drops: {e}")

def main():
    print("Running fetch_data...")
    fetch_data()
    
    print("Running fetch_drops...")
    fetch_drops()

if __name__ == "__main__":
    main()