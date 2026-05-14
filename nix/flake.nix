{
  description = "Vellum Core Toolchain";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    vellumConfig = import ./config.nix;
    vellumPackages = import ./packages.nix { inherit pkgs; };

  in {
    packages.${system} = {
      vellum-deps = pkgs.symlinkJoin {
        name = "vellum-deps";
        paths = vellumPackages;
      };
    };

    lib = {
      splitCueImage = { name ? "split", cue, image }: pkgs.stdenv.mkDerivation {
        inherit name cue image;
        buildInputs = vellumPackages;
        unpackPhase = "true";
        buildPhase = ''
          mkdir -p $out
          shnsplit -f "$cue" -o flac -t "%n" -d $out "$image"
        '';
        installPhase = "true";
      };

      mkCover = { name, src, relPath ? null }: pkgs.stdenv.mkDerivation {
        inherit name src relPath;
        buildInputs = vellumPackages;
        unpackPhase = "true";
        buildPhase = ''
          INPUT_FILE="${if relPath == null then "$src" else "$src/$relPath"}"
          magick "$INPUT_FILE" -filter Mitchell -thumbnail 1080x1080^ -gravity center -extent 1080x1080 cover.png
        '';
        installPhase = ''
          mkdir -p $out
          cp cover.png $out/cover.png
        '';
      };

      mkTrack = { name, src, relPath, metadata ? {}, cover ? null }: let
        filteredMeta = pkgs.lib.filterAttrs (k: v: builtins.elem (pkgs.lib.toLower k) vellumConfig.allowedTags) metadata;
        metaJson = pkgs.writeText "meta.json" (builtins.toJSON filteredMeta);
      in pkgs.stdenv.mkDerivation {
        inherit name src relPath;
        buildInputs = vellumPackages;
        unpackPhase = "true";
        buildPhase = ''
          cp "$src/$relPath" track.flac
          chmod +w track.flac
          metaflac --remove-all-tags track.flac
          
          jq -r 'to_entries | .[] | if (.value | type) == "array" then .key as $k | .value[] | "\($k)=\(.)" else "\(.key)=\(.value)" end' ${metaJson} > tags.txt
          while IFS= read -r tag; do
            metaflac --set-tag="$tag" track.flac
          done < tags.txt

          ${if cover != null then ''metaflac --import-picture-from="${cover}/cover.png" track.flac'' else ""}
        '';
        installPhase = ''
          mkdir -p $out
          cp track.flac $out/track.flac
        '';
      };

      mkAlbum = { 
        pname, 
        sourceDisk ? { hash = ""; },
        sourceTorrent ? { hash = ""; },
        album ? {},
        tracks ? [], 
        cover ? null
      }: let
        getMergedTrackMeta = t: let
          attrKeys = builtins.filter (k: builtins.isAttrs (album.${k} or null) || builtins.isAttrs (t.${k} or null)) (pkgs.lib.unique (builtins.attrNames album ++ builtins.attrNames t));
        in pkgs.lib.foldl' (acc: n: acc // (if builtins.isAttrs (album.${n} or null) then album.${n} else {}) // (if builtins.isAttrs (t.${n} or null) then t.${n} else {})) {} attrKeys;

        trackIds = builtins.map (t: let m = getMergedTrackMeta t; in "${toString (m.discnumber or 1)}-${toString (m.tracknumber or 0)}") tracks;
        uniqueTrackIds = pkgs.lib.unique trackIds;
        hasDuplicates = builtins.length trackIds != builtins.length uniqueTrackIds;

        maxDisc = builtins.foldl' (acc: t: pkgs.lib.max acc ((getMergedTrackMeta t).discnumber or 1)) 1 tracks;
        maxTrack = builtins.foldl' (acc: t: pkgs.lib.max acc ((getMergedTrackMeta t).tracknumber or 0)) 1 tracks;
        
        discPadLen = builtins.stringLength (toString maxDisc);
        trackPadLen = pkgs.lib.max 2 (builtins.stringLength (toString maxTrack));

        toTomlVal = v:
          if builtins.isString v then "\"${pkgs.lib.escape ["\"" "\\"] v}\""
          else if builtins.isInt v then toString v
          else if builtins.isBool v then (if v then "true" else "false")
          else if builtins.isList v then "[ " + pkgs.lib.concatMapStringsSep ", " toTomlVal v + " ]"
          else "\"\"";
        
        toTomlTable = order: data: let
          orderedLines = pkgs.lib.concatMap (pathStr:
            if pathStr == "\n" then [ "" ]
            else let
              parts = pkgs.lib.splitString "." pathStr;
              manifest = builtins.elemAt parts 0;
              key = builtins.elemAt parts 1;
            in if builtins.isAttrs (data.${manifest} or null) && data.${manifest} ? ${key}
               then [ "${key} = ${toTomlVal data.${manifest}.${key}}" ]
               else []
          ) order;
          
          rawLines = orderedLines;
          cleanLines = pkgs.lib.foldl' (acc: x: 
            if x != "" 
            then acc ++ [x] 
            else if (acc != [] && pkgs.lib.last acc == "") 
            then acc 
            else acc ++ [x]
          ) [] rawLines;
          tightLines = if cleanLines != [] && pkgs.lib.last cleanLines == "" then pkgs.lib.init cleanLines else cleanLines;
        in pkgs.lib.concatStringsSep "\n" tightLines;

        metadataToml = let
          aTable = toTomlTable vellumConfig.keys.album album;
          aS = if aTable != "" then "[album]\n${aTable}" else "";
          tS = pkgs.lib.concatMapStringsSep "\n\n" (t: let table = toTomlTable vellumConfig.keys.tracks t; in if table != "" then "[[tracks]]\n${table}" else "[[tracks]]") tracks;
          sep = if aS != "" && tS != "" then "\n\n" else "";
        in pkgs.writeText "metadata.toml" (aS + sep + tS + "\n");

        stagingSrc = builtins.getEnv "VELLUM_STAGING_SRC";

        rawSrcPath = if stagingSrc != "" then stagingSrc
                     else throw "VELLUM_STAGING_SRC must be set during build initialization";

        envTorrentName = builtins.getEnv "VELLUM_TORRENT_NAME";
        srcBaseName = if envTorrentName != "" then envTorrentName
                      else if (sourceTorrent.name or "") != "" then sourceTorrent.name
                      else pname;

        realSrc = 
          let
            storeMatch = builtins.match ".*(/nix/store/[0-9abcdfghijklmnpqrsvwxyz]{32}-.*)" rawSrcPath;
            rawVal = if storeMatch != null then builtins.elemAt storeMatch 0 else rawSrcPath;
            isStore = builtins.isString rawVal && builtins.match "/nix/store/.*" rawVal != null;
          in
          if isStore then 
            builtins.storePath rawVal
          else if builtins.isPath rawVal then 
            builtins.path { 
              name = "${srcBaseName}-source"; 
              path = rawVal; 
              sha256 = sourceDisk.hash; 
            }
          else 
            rawVal;

        processedCover = if cover != null && cover.file != null
                         then (
                           if builtins.isPath cover.file
                           then let
                             realCoverPath = if (cover.hash or "") != "" then
                               builtins.path { 
                                 name = "${pname}-cover-src"; 
                                 path = cover.file; 
                                 sha256 = cover.hash; 
                                 recursive = false;
                               }
                             else cover.file;
                           in self.lib.mkCover { name = "${pname}-cover"; src = realCoverPath; }
                           else self.lib.mkCover { name = "${pname}-cover"; src = realSrc; relPath = cover.file; }
                         )
                         else null;
        
        builtTracks = pkgs.lib.lists.imap1 (idx: track: let
          mergedMeta = getMergedTrackMeta track;
          disc = mergedMeta.discnumber or 1;
          trk = mergedMeta.tracknumber or 0;
          title = mergedMeta.title or "Untitled";
          discStr = pkgs.lib.fixedWidthString discPadLen "0" (toString disc);
          trkStr = pkgs.lib.fixedWidthString trackPadLen "0" (toString trk);
          fileName = if maxDisc == 1 then "${trkStr} - ${title}.flac" else "${discStr}.${trkStr} - ${title}.flac";
        in {
          inherit fileName;
          drv = self.lib.mkTrack {
            name = "${pname}-disc${toString disc}-track${toString trk}";
            src = realSrc;
            relPath = track.file;
            metadata = mergedMeta;
            cover = processedCover;
          };
        }) tracks;

      in if hasDuplicates then throw "Duplicate discnumber and tracknumber combinations found in tracks." else pkgs.stdenv.mkDerivation {
        name = pname;
        src = realSrc;
        passthru = {
          sourceStorePath = realSrc;
        };
        unpackPhase = "true";
        buildPhase = ''
          mkdir -p $out
          ${pkgs.lib.strings.concatMapStringsSep "\n" (t: ''
            ln -s "${t.drv}/track.flac" "$out/${t.fileName}"
          '') builtTracks}
          cp ${metadataToml} $out/metadata.toml
        '';
        installPhase = "true";
      };
    };
  };
}
