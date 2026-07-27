# Homebrew formula for the anesis CLI.
#
# Rendered from packaging/homebrew/anesis.rb by packaging/render.sh and pushed to
# anesis-dev/homebrew-anesis as Formula/anesis.rb. Do not edit the copy in the
# tap by hand — the next release overwrites it.
#
# Ships prebuilt binaries rather than building from source: the crate is
# PolyForm-Noncommercial, and Homebrew's source builds are not the distribution
# path this project supports.
class Anesis < Formula
  desc "Scaffold projects from remote templates and extend them with addons"
  homepage "https://anesis-dev.vercel.app"
  version "{{VERSION}}"
  license :cannot_represent # PolyForm-Noncommercial-1.0.0

  on_macos do
    on_arm do
      url "https://github.com/anesis-dev/anesis-cli/releases/download/v{{VERSION}}/anesis-macos-aarch64.tar.gz"
      sha256 "{{SHA256_MACOS_AARCH64}}"
    end
    on_intel do
      url "https://github.com/anesis-dev/anesis-cli/releases/download/v{{VERSION}}/anesis-macos-x86_64.tar.gz"
      sha256 "{{SHA256_MACOS_X86_64}}"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/anesis-dev/anesis-cli/releases/download/v{{VERSION}}/anesis-linux-aarch64.tar.gz"
      sha256 "{{SHA256_LINUX_AARCH64}}"
    end
    on_intel do
      url "https://github.com/anesis-dev/anesis-cli/releases/download/v{{VERSION}}/anesis-linux-x86_64.tar.gz"
      sha256 "{{SHA256_LINUX_X86_64}}"
    end
  end

  def install
    bin.install "anesis"

    # The binary generates both of these itself, so the formula never carries a
    # stale copy. `--print` matters: without it `completions` installs into the
    # *building* user's dotfiles rather than into the prefix.
    system bin/"anesis", "man", buildpath/"man"
    man1.install Dir[buildpath/"man/*.1"]

    (bash_completion/"anesis").write Utils.safe_popen_read(bin/"anesis", "completions", "bash", "--print")
    (zsh_completion/"_anesis").write Utils.safe_popen_read(bin/"anesis", "completions", "zsh", "--print")
    (fish_completion/"anesis.fish").write Utils.safe_popen_read(bin/"anesis", "completions", "fish", "--print")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/anesis --version")
  end
end
