class McpProbe < Formula
  desc "Interactive CLI debugger and TUI for MCP (Model Context Protocol) servers"
  homepage "https://github.com/conikeec/mcp-probe"
  version "0.1.0"
  
  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/conikeec/mcp-probe/releases/download/v0.1.0/mcp-probe-x86_64-apple-darwin.tar.gz"
      sha256 "UPDATE_ME" # This will be automatically updated by the release workflow
    else
      url "https://github.com/conikeec/mcp-probe/releases/download/v0.1.0/mcp-probe-aarch64-apple-darwin.tar.gz"
      sha256 "UPDATE_ME" # This will be automatically updated by the release workflow
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/conikeec/mcp-probe/releases/download/v0.1.0/mcp-probe-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "UPDATE_ME" # This will be automatically updated by the release workflow
    else
      url "https://github.com/conikeec/mcp-probe/releases/download/v0.1.0/mcp-probe-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "UPDATE_ME" # This will be automatically updated by the release workflow
    end
  end

  def install
    bin.install "mcp-probe"
    
    # Generate shell completions if the binary supports it
    # Note: This assumes the binary will support completion generation in the future
    if (bin/"mcp-probe").exist?
      generate_completions_from_executable(bin/"mcp-probe", "completion")
    end
  end

  test do
    # Test that the binary runs and shows version
    assert_match(/mcp-probe/, shell_output("#{bin}/mcp-probe --version"))
    
    # Test help output
    help_output = shell_output("#{bin}/mcp-probe --help")
    assert_match(/Model Context Protocol/, help_output)
    assert_match(/debug/, help_output)
  end

  def caveats
    <<~EOS
      🚀 MCP Probe has been installed!
      
      📖 Quick start:
        mcp-probe --help
        mcp-probe debug --http-sse http://localhost:3000/sse
      
      📚 Documentation: https://github.com/conikeec/mcp-probe
      
      💡 For advanced usage and examples, see:
        https://github.com/conikeec/mcp-probe/blob/main/README.md
    EOS
  end
end 