#!/usr/bin/env python3
import os
import sys
import json
import re
import socket
import urllib.request
import urllib.parse
from pathlib import Path
import lyricsgenius

def get_playing_file():
    host = os.environ.get("MPD_HOST", "127.0.0.1")
    port = int(os.environ.get("MPD_PORT", "6600"))
    try:
        s = socket.create_connection((host, port), timeout=2)
        f = s.makefile("rw")
        f.readline()
        f.write("currentsong\n")
        f.flush()
        file_path = None
        for line in f:
            if line.startswith("file: "):
                file_path = line[6:].strip()
            elif line.startswith("OK") or line.startswith("ACK"):
                break
        s.close()
        return file_path
    except Exception:
        return None

def trigger_update(album_id):
    encoded_id = urllib.parse.quote(album_id, safe='')
    url = f"http://127.0.0.1:8000/api/update-album/{encoded_id}"
    req = urllib.request.Request(url, method="POST")
    try:
        urllib.request.urlopen(req, timeout=2)
    except Exception:
        pass

def clean_genius_lyrics(lyrics, title):
    if not lyrics:
        return ""
    
    lines = lyrics.split("\n")
    if lines and "Contributors" in lines[0]:
        lines.pop(0)
    
    filtered_lines = []
    for line in lines:
        trimmed = line.strip()
        if trimmed.startswith("[") and trimmed.endswith("]"):
            filtered_lines.append("")
            continue
        filtered_lines.append(trimmed)
    
    cleaned = "\n".join(filtered_lines)
    
    cleaned = re.sub(r"\(\s*\n\s*", "(", cleaned)
    cleaned = re.sub(r"\s*\n\s*\)", ")", cleaned)
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned)
    
    cleaned = re.sub(r"[0-9]*Embed$", "", cleaned)
    cleaned = cleaned.strip()
    
    return cleaned

def sanitize_filename(name):
    return re.sub(r'[<>:"/\\|?*]', '_', name)

def get_album_lyrics(vellum_cfg, album_lock, access_token, mpd_file):
    library_str = vellum_cfg.get("storage", {}).get("library", "")
    if not library_str:
        print("Error: library not defined in config")
        return

    library = Path(library_str).expanduser().resolve()
    
    album_meta = album_lock.get("album", {})
    album_id = album_meta.get("id", "")
    if not album_id:
        print("Error: album_path (id) not found in metadata lock")
        return

    root = (library / album_id).resolve()
    album_artist = album_meta.get("albumartist")
    total_discs = int(album_meta.get("info", {}).get("total_discs", 1))
    tracks = album_lock.get("tracks", [])

    if not album_artist or not tracks:
        print("Error: Invalid metadata structure in lock data.")
        return

    genius = lyricsgenius.Genius(access_token)
    genius.verbose = False
    genius.remove_section_headers = False

    lyrics_dir = root / "Lyrics"
    lyrics_dir.mkdir(exist_ok=True)

    print(f"Fetching lyrics for: {album_artist} - {album_meta.get('album')}")

    playing_idx = None
    if mpd_file:
        for i, track in enumerate(tracks):
            t_path = track.get("file", {}).get("path", "")
            if t_path:
                full_rel = f"{album_id}/{t_path}"
                if full_rel == mpd_file:
                    playing_idx = i
                    break

    def fetch_for_track(track):
        title = track.get("title")
        track_num = str(track.get("tracknumber", "0")).zfill(2)
        disc_num = str(track.get("discnumber", "1"))
        
        if not title:
            return

        safe_title = sanitize_filename(title)

        if total_discs > 1:
            filename = f"{disc_num}.{track_num} - {safe_title}.txt"
        else:
            filename = f"{track_num} - {safe_title}.txt"
            
        dest_path = lyrics_dir / filename

        if dest_path.exists():
            print(f"  Skipping: {title} (File exists)")
            return

        try:
            song = genius.search_song(title, album_artist)
            if song:
                cleaned_text = clean_genius_lyrics(song.lyrics, title)
                with open(dest_path, "w", encoding="utf-8") as lf:
                    lf.write(cleaned_text)
                print(f"  Saved: {title}")
            else:
                print(f"  Not found: {title}")
        except Exception as e:
            print(f"  Error fetching {title}: {e}")

    if playing_idx is not None:
        fetch_for_track(tracks[playing_idx])
        trigger_update(album_id)

    for i, track in enumerate(tracks):
        if i == playing_idx:
            continue
        fetch_for_track(track)

    trigger_update(album_id)

def main():
    try:
        data = json.load(sys.stdin)
    except Exception as e:
        print(f"Error reading JSON from stdin: {e}")
        sys.exit(1)

    albums = data.get("albums", [])
    vellum_cfg = data.get("config", {}).get("vellum", {})
    action_cfg = data.get("config", {}).get("action", {})

    token = os.environ.get("GENIUS_ACCESS_TOKEN") or os.environ.get("GENIUS_API_KEY")
    if not token:
        token = vellum_cfg.get("actions", {}).get("genius_access_token")
    if not token:
        token = action_cfg.get("genius_access_token")
    
    if not token:
        print("Error: Genius Access Token is required.")
        sys.exit(1)

    mpd_file = get_playing_file()

    for album_lock in albums:
        get_album_lyrics(vellum_cfg, album_lock, token, mpd_file)

if __name__ == "__main__":
    main()
