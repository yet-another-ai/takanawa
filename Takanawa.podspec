Pod::Spec.new do |s|
  version = "0.1.0"
  release_base_url = "https://github.com/yetanother.ai/takanawa/releases/download/v#{version}"
  xcframework_url = ENV.fetch("TAKANAWA_XCFRAMEWORK_URL", "#{release_base_url}/Takanawa.xcframework.zip")
  xcframework_sha256 = ENV["TAKANAWA_XCFRAMEWORK_SHA256"]

  s.name = "Takanawa"
  s.version = version
  s.summary = "Rust range-download library with a stable C ABI."
  s.description = <<~DESC
    Takanawa is a Rust range-download library designed to ship as a C ABI
    library on Apple platforms. It stores download state in a .part file so
    interrupted downloads can resume automatically.
  DESC
  s.homepage = "https://github.com/yetanother.ai/takanawa"
  s.license = { :type => "MIT OR Apache-2.0", :file => "LICENSE" }
  s.author = { "yetanother.ai" => "opensource@yetanother.ai" }
  s.module_name = "Takanawa"

  s.platforms = {
    :ios => "13.0",
    :osx => "10.15"
  }

  s.source = if xcframework_sha256 && !xcframework_sha256.empty?
    { :http => xcframework_url, :sha256 => xcframework_sha256 }
  else
    { :http => xcframework_url }
  end

  s.vendored_frameworks = "Takanawa.xcframework"
  s.static_framework = true
  s.requires_arc = true
end
