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
              gst_all_1.gst-plugins-base
              gst_all_1.gst-plugins-good
              gst_all_1.gst-plugins-bad
              gst_all_1.gst-plugins-ugly
              gst_all_1.gst-libav
              gst_all_1.gst-vaapi

              alsa-lib
              cairo
            ];
          };
        }
      );
}
