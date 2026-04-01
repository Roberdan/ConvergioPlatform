# Homebrew formula for Convergio Platform
# Install: brew tap Roberdan/convergio && brew install convergio
# Update: scripts/release/update-homebrew.sh <version>

class Convergio < Formula
  desc "AI orchestration platform — 69 agents, 12 domains, local-first"
  homepage "https://convergio.io"
  version "20.4.0"
  license "Convergio-Community"

  on_arm do
    url "https://github.com/Roberdan/ConvergioPlatform/releases/download/v20.4.0/convergio-v20.4.0-aarch64-apple-darwin.tar.gz"
    sha256 "PLACEHOLDER_ARM64_SHA256"
  end

  on_intel do
    url "https://github.com/Roberdan/ConvergioPlatform/releases/download/v20.4.0/convergio-v20.4.0-x86_64-apple-darwin.tar.gz"
    sha256 "PLACEHOLDER_X86_64_SHA256"
  end

  def install
    bin.install "cvg"
    bin.install_symlink "cvg" => "convergio-platform-daemon"
  end

  def caveats
    config = "#{Dir.home}/.convergio"
    return if Dir.exist?(config)

    <<~EOS
      To complete setup, run:
        cvg setup

      This creates ~/.convergio with default configuration.
      Documentation: https://convergio.io/docs
    EOS
  end

  test do
    assert_match(/\d+\.\d+\.\d+/, shell_output("#{bin}/cvg --version"))
  end
end
