{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          ffmpeg = pkgs.ffmpeg_6;
        in
        {
          devShell = with pkgs; mkShell {
            LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
            buildInputs = [
              pkg-config
              openssl


              gst_all_1.gstreamer
              # Common plugins like "filesrc" to combine within e.g. gst-launch
              gst_all_1.gst-plugins-base
              # Specialized plugins separated by quality
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad
              gst_all_1.gst-plugins-ugly
              # Plugins to reuse ffmpeg to play almost every video format
              gst_all_1.gst-libav
              # Support the Video Audio (Hardware) Acceleration API
              gst_all_1.gst-vaapi
            ];
          };
        }
      );
}
