# The NDK's CMake toolchain for 64-bit ARM phones (#240). Used for crates
# that build C/C++ with CMake (whisper.cpp via whisper-rs-sys): cmake-rs
# passes only CMAKE_TOOLCHAIN_FILE, and the NDK file alone defaults to
# 32-bit armeabi-v7a, which then fails to link into the arm64 app.
# Point CMAKE_TOOLCHAIN_FILE_aarch64_linux_android here (docs/android.md).
set(ANDROID_ABI arm64-v8a CACHE STRING "" FORCE)
set(ANDROID_PLATFORM android-26 CACHE STRING "" FORCE)
include($ENV{NDK_HOME}/build/cmake/android.toolchain.cmake)
