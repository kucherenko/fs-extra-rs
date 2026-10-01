fn main() {
  // Platform link args so the napi_* symbols resolve at module load time
  // (standard napi-rs addon setup).
  napi_build::setup();
}
