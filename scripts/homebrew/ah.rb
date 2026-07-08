class Ah < Formula
  desc "Terminal translate & explain — select a word, get translation + AI explanation"
  homepage "https://github.com/USERNAME_TODO/ah"
  version "USERNAME_TODO"

  if OS.linux?
    if Hardware::CPU.arm?
      url "https://github.com/USERNAME_TODO/ah/releases/download/vUSERNAME_TODO/ah-linux-aarch64.tar.gz"
      sha256 "USERNAME_TODO"
    else
      url "https://github.com/USERNAME_TODO/ah/releases/download/vUSERNAME_TODO/ah-linux-x86_64.tar.gz"
      sha256 "USERNAME_TODO"
    end
  elsif OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/USERNAME_TODO/ah/releases/download/vUSERNAME_TODO/ah-macos-aarch64.tar.gz"
      sha256 "USERNAME_TODO"
    else
      url "https://github.com/USERNAME_TODO/ah/releases/download/vUSERNAME_TODO/ah-macos-x86_64.tar.gz"
      sha256 "USERNAME_TODO"
    end
  end

  def install
    bin.install "ah"
  end

  test do
    assert_match "ah", shell_output("#{bin}/ah --help")
  end
end
