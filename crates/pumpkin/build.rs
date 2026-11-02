// Source livrée sans historique Git : le commit Pumpkin de base est fixé ici.
const BASE_COMMIT: &str = "204a94ed895f041a845630d5a23c094705a0704e";

fn main() {
    println!("cargo::rustc-env=GIT_HASH={}", &BASE_COMMIT[..7]);
}
