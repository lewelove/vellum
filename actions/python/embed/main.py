#!/usr/bin/env python3
import os
import sys
import json
import base64
import mimetypes
from pathlib import Path

import xxhash
from mutagen.flac import FLAC, Picture

SYNC_TAGS = [
    "ALBUM", "ALBUMARTIST", "DATE", "GENRE", "COMMENT",
    "TITLE", "ARTIST", "DISCOGS_URL", "MUSICBRAINZ_URL",
    "REPLAYGAIN_TRACK_GAIN", "REPLAYGAIN_ALBUM_GAIN"
]

def get_hash(data):
    h = xxhash.xxh64(data).digest()
    return base64.urlsafe_b64encode(h).decode("ascii").rstrip("=")

def process_album(album_lock, target_dir, auto_apply):
    album_obj = album_lock.get("album", {})
    tracks_data = album_lock.get("tracks", [])
    if not tracks_data:
        return

    exclude_keys = {"info", "tags", "keys", "covers", "manifests", "file", "id"}
    album_pool = {k.upper(): v for k, v in album_obj.items() if k not in exclude_keys and not isinstance(v, dict)}
    for pool_key in ["tags", "keys"]:
        if pool_key in album_obj:
            album_pool.update({k.upper(): v for k, v in album_obj[pool_key].items()})

    total_discs = int(album_obj.get("info", {}).get("total_discs", 1))
    
    cover_filename = album_obj.get("covers", {}).get("main", {}).get("file", {}).get("path", "cover.png")
    cover_path = target_dir / cover_filename
    
    disk_cover_data = None
    disk_cover_hash = None
    if cover_path.exists():
        with open(cover_path, "rb") as cf:
            disk_cover_data = cf.read()
        disk_cover_hash = get_hash(disk_cover_data)

    first_track_path = target_dir / tracks_data[0].get("file", {}).get("path", "")
    if not first_track_path.exists():
        return
        
    first_audio = FLAC(first_track_path)
    embedded_cover_hash = None
    if first_audio.pictures:
        for pic in first_audio.pictures:
            if pic.type == 3:
                embedded_cover_hash = get_hash(pic.data)
                break

    update_cover = (disk_cover_hash is not None) and (embedded_cover_hash != disk_cover_hash)

    tasks = []
    for t in tracks_data:
        t_file = t.get("file", {})
        rel_path = t_file.get("path")
        if not rel_path:
            continue

        target_tags = {}
        track_pool = {k.upper(): v for k, v in t.items() if k not in exclude_keys and not isinstance(v, dict)}
        for pool_key in ["tags", "keys"]:
            if pool_key in t:
                track_pool.update({k.upper(): v for k, v in t[pool_key].items()})

        for tag_name in SYNC_TAGS:
            val = track_pool.get(tag_name, album_pool.get(tag_name))
            if val is not None:
                target_tags[tag_name] = "; ".join(val) if isinstance(val, list) else str(val)

        target_tags["ARTIST"] = str(track_pool.get("ARTIST", ""))
        target_tags["TITLE"] = str(track_pool.get("TITLE", ""))
        target_tags["TRACKNUMBER"] = str(track_pool.get("TRACKNUMBER", "0"))
        
        if total_discs > 1:
            target_tags["DISCNUMBER"] = str(track_pool.get("DISCNUMBER", "1"))
    
        tasks.append({
            "path": rel_path,
            "target_tags": target_tags,
            "diffs": []
        })

    for task in tasks:
        track_file = target_dir / task["path"]
        if not track_file.exists():
            continue
            
        audio = FLAC(track_file)
        target_keys = set(task["target_tags"].keys())
        diffs = []
        
        for old_tag in list(audio.keys()):
            u_old = old_tag.upper()
            if u_old not in target_keys:
                old_vals = audio.get(old_tag, [])
                old_val = "; ".join(old_vals) if isinstance(old_vals, list) else str(old_vals)
                diffs.append(f"\033[31m- {u_old}: \033[90m{old_val}\033[0m")

        for tag, new_val in task["target_tags"].items():
            old_vals = audio.get(tag, audio.get(tag.lower(), []))
            old_val = old_vals[0] if old_vals else ""
            if str(old_val) != str(new_val):
                if not old_val:
                    diffs.append(f"\033[32m+ {tag}: \033[90m{new_val}\033[0m")
                else:
                    diffs.append(f"\033[34m~ {tag}: \033[90m{old_val} -> {new_val}\033[0m")

        task["diffs"] = diffs

    active_tasks = [t for t in tasks if t["diffs"]]
    common_diffs = []
    if len(active_tasks) > 1:
        first_diffs = active_tasks[0]["diffs"]
        for d in first_diffs:
            if all(d in t["diffs"] for t in active_tasks):
                common_diffs.append(d)
                
        for t in active_tasks:
            t["diffs"] = [d for d in t["diffs"] if d not in common_diffs]

    has_actual_changes = update_cover or bool(common_diffs) or any(t["diffs"] for t in active_tasks)
    if not has_actual_changes:
        return

    print(f"\n\033[1;36m{target_dir.name}\033[0m")

    if update_cover:
        print("\033[33m🖼️  Cover update required\033[0m")

    if common_diffs:
        print("\033[1;34m💿 Album Diff\033[0m")
        for d in common_diffs:
            print(f"   {d}")

    for task in tasks:
        if task["diffs"]:
            print(f"\033[1m🎵 {task['path']}\033[0m")
            for d in task["diffs"]:
                print(f"   {d}")

    if not auto_apply:
        try:
            sys.stdout.write(f"\n\033[1;35mApply changes? [y/N]: \033[0m")
            sys.stdout.flush()
            with open('/dev/tty', 'r') as tty:
                ans = tty.readline().strip().lower()
            if ans not in ('y', 'yes'):
                return
        except Exception:
            return

    new_pic = None
    if update_cover:
        new_pic = Picture()
        new_pic.data = disk_cover_data
        new_pic.type = 3
        new_pic.mime = mimetypes.guess_type(cover_path)[0] or "image/jpeg"
        new_pic.desc = "Front Cover"

    for task in tasks:
        rel_path = task["path"]
        audio = FLAC(target_dir / rel_path)
        
        target_tags = task["target_tags"]
        for old_tag in list(audio.keys()):
            if old_tag.upper() not in target_tags:
                del audio[old_tag]
        for tag, val in target_tags.items():
            audio[tag] = [val]
        
        if update_cover:
            audio.clear_pictures()
            audio.add_picture(new_pic)
            
        audio.save()
        
    print("\033[32m✔ Done.\033[0m")


def main():
    try:
        data = json.load(sys.stdin)
    except Exception as e:
        print(f"Error reading JSON from stdin: {e}")
        sys.exit(1)

    albums = data.get("albums", [])
    vellum_cfg = data.get("config", {}).get("vellum", {})
    action_cfg = data.get("config", {}).get("action", {})
    options_str = data.get("options", "")

    auto_apply = "--auto" in options_str or "-y" in options_str

    if "sync_tags" in action_cfg:
        global SYNC_TAGS
        SYNC_TAGS = action_cfg["sync_tags"]

    library_str = vellum_cfg.get("storage", {}).get("library", "")
    if not library_str:
        print("Error: library not defined in config")
        sys.exit(1)

    library = Path(library_str).expanduser().resolve()

    for album_lock in albums:
        album_id = album_lock.get("album", {}).get("id", "")
        if not album_id:
            continue
        
        target_dir = library / album_id
        process_album(album_lock, target_dir, auto_apply)

if __name__ == "__main__":
    main()
