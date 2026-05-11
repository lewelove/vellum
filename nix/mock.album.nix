{ vellum }:

vellum.mkAlbum {

  pname = "";

  sourceTorrent = {
    file = ./Info/source.torrent;
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  sourceDisk.hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

  cover = {
    file = ./cover.png;
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  album = {
    metadata = {
      albumartist = "";
      album = "";
      date = "";
      genre = "";
    };
    mbid = {
    };
  };

  tracks = [
    {
      file = "";
      metadata = {
        tracknumber = 1;
        title = "";
      };
      mbid = {
      };
    }
  ];
}

