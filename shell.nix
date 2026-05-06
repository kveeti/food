let
	pkgs = import <nixpkgs> { };
in
pkgs.mkShell {
	nativeBuildInputs = with pkgs; [
		nodejs_24
		pnpm

		caddy
		tailwindcss_4

		rustup

		bubblewrap
	];

	shellHook = ''
		export PATH="$HOME/.bun/bin:$PATH"
	'';
}

