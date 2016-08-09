with import <nixpkgs> {}; {
 rust_glEnv = stdenv.mkDerivation {
   name = "rust-glEnv";
   shellHook = ''
    export LD_LIBRARY_PATH="${lib.makeLibraryPath
      [ stdenv rustc xlibsWrapper
        xorg.libX11 xorg.libXcursor xorg.libXxf86vm xorg.libXi
      ]}:$LD_LIBRARY_PATH";
    '';
   buildInputs = [ stdenv rustc xlibsWrapper cargo
                   xorg.libX11 xorg.libXcursor xorg.libXxf86vm xorg.libXi ];
 };
}
