#!/usr/bin/env python3
import sys
import json
import re
import urllib.request
import urllib.parse
from pathlib import Path

def trigger_update(album_id):
    encoded_id = urllib.parse.quote(album_id, safe='')
    url = f"http://127.0.0.1:8000/api/update-album/{encoded_id}"
    req = urllib.request.Request(url, method="POST")
    try:
        urllib.request.urlopen(req, timeout=2)
    except Exception:
        pass

def sanitize_filename(name):
    return re.sub(r'[<>:"/\\|?*]', '_', name)

def process_album(album_lock, library, auto_apply):
    album_obj = album_lock.get("album", {})
    tracks_data = album_lock.get("tracks", [])
    if not tracks_data:
        return

    album_id = album_obj.get("id", "")
    if not album_id:
        return

    target_dir = library / album_id
    if not target_dir.exists():
        return

    try:
        total_discs = int(album_obj.get("info", {}).get("total_discs", 1))
    except (ValueError, TypeError):
        total_discs = 1

    track_nums = []
    disc_nums = []
    for t in tracks_data:
        try:
            track_nums.append(int(t.get("tracknumber", 0)))
        except (ValueError, TypeError):
            track_nums.append(0)
        try:
            disc_nums.append(int(t.get("discnumber", 1)))
        except (ValueError, TypeError):
            disc_nums.append(1)

    max_track_num = max(track_nums) if track_nums else 0
    max_disc_num = max(disc_nums + [total_discs]) if disc_nums else 1

    track_pad = max(2, len(str(max_track_num)))
    disc_pad = max(1, len(str(max_disc_num)))

    rename_tasks = []

    for t in tracks_data:
        t_file = t.get("file", {})
        rel_path = t_file.get("path")
        if not rel_path:
            continue

        old_file_path = target_dir / rel_path
        if not old_file_path.exists():
            continue

        try:
            track_num = int(t.get("tracknumber", 0))
        except (ValueError, TypeError):
            track_num = 0

        try:
            disc_num = int(t.get("discnumber", 1))
        except (ValueError, TypeError):
            disc_num = 1

        title = str(t.get("title", ""))
        safe_title = sanitize_filename(title)
        ext = old_file_path.suffix

        track_str = str(track_num).zfill(track_pad)

        if total_discs >= 2:
            disc_str = str(disc_num).zfill(disc_pad)
            new_filename = f"{disc_str}.{track_str} - {safe_title}{ext}"
        else:
            new_filename = f"{track_str} - {safe_title}{ext}"

        new_file_path = old_file_path.parent / new_filename

        if old_file_path != new_file_path:
            rename_tasks.append({
                "old_path": old_file_path,
                "new_path": new_file_path,
                "rel_path": rel_path,
                "old_name": old_file_path.name,
                "new_name": new_filename
            })

    if not rename_tasks:
        return

    print(f"\n\033[1;36m{target_dir.name}\033[0m")
    for task in rename_tasks:
        print(f"\033[1m🎵 {task['rel_path']}\033[0m")
        print(f"   \033[34m~ {task['old_name']} -> {task['new_name']}\033[0m")

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

    temp_tasks = []
    for idx, task in enumerate(rename_tasks):
        old_p = task["old_path"]
        new_p = task["new_path"]
        temp_p = old_p.with_name(f"{old_p.name}.tmp_rename_{idx}")
        old_p.rename(temp_p)
        temp_tasks.append((temp_p, new_p))

    for temp_p, new_p in temp_tasks:
        temp_p.rename(new_p)

    print("\033[32m✔ Done.\033[0m")
    trigger_update(album_id)

def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(1)

    albums = data.get("albums", [])
    vellum_cfg = data.get("config", {}).get("vellum", {})
    options_str = data.get("options", "")

    auto_apply = "--auto" in options_str or "-y" in options_str

    library_str = vellum_cfg.get("storage", {}).get("library", "")
    if not library_str:
        sys.exit(1)

    library = Path(library_str).expanduser().resolve()

    for album_lock in albums:
        process_album(album_lock, library, auto_apply)

if __name__ == "__main__":
    main()
