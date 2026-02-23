class ClipShow < Formula
  desc "Show copied clipboard text as a HUD on macOS"
  homepage "https://github.com/somei-san/clip-show"
  url "https://github.com/somei-san/clip-show/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "9e4b47e27bbfd52506f15f73acff742a5693295702bbcbb2148e031ae8609480"
  license "MIT"
  head "https://github.com/somei-san/clip-show.git", branch: "main"

  depends_on :macos
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  service do
    run [opt_bin/"clip-show"]
    keep_alive true
    run_at_load true
    process_type :interactive
    log_path var/"log/clip-show.log"
    error_log_path var/"log/clip-show.log"
  end

  test do
    assert_equal version.to_s, shell_output("#{bin}/clip-show --version").strip
  end
end
