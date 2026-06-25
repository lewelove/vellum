import sys
import json
import urllib.parse
import subprocess

def main():
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(1)

    albums = data[0]
    
    for album_lock in albums:
        album_meta = album_lock.get("album", {})
        
        album_title = album_meta.get("album", "")
        album_artist = album_meta.get("albumartist", "")

        artist_encoded = urllib.parse.quote(album_artist)
        album_encoded = urllib.parse.quote(album_title)

        url = f"https://covers.musichoarders.xyz/?theme=dark&sources=amazonmusic,applemusic,deezer,discogs,fanarttv,lastfm,musicbrainz,qobuz,soulseek&country=US&artist={artist_encoded}&album={album_encoded}"

        subprocess.Popen(
            ["chromium-browser", f"--app={url}"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL
        )

if __name__ == "__main__":
    main()
