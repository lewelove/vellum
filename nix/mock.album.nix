{ vellum }:

vellum.mkAlbum {

  pname = "";

  sourceTorrent = {
    file = ./Info/source.torrent;
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
  };

  sourceDisk.hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

  album = {
    metadata = {
      albumartist = "";
      album = "";
      date = "";
      genre = "";
    };
  };

  cover = ./cover.png;

  tracks = [
    {
      file = "";
      metadata = {
        tracknumber = 1;
        title = "";
      };
    }
  ];
}

