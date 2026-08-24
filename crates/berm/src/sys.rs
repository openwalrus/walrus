//! The host half of the `crabtalk` namespace: one constructor per name, taking
//! the implementation and returning the [`berm::Harness`] that serves it.
//!
//! `berm-crabtalk` declares the guest half of the same thing in its own crate.

berm::hosts! {
    namespace = "crabtalk";

    /// Files, bounded by a granted root.
    mod fs {
        /// Read a file whole.
        fn read(path: &str) -> Vec<u8>;
        /// Write a file, replacing what was there.
        fn write(path: &str, content: &[u8]);
        /// Paths under `path` matching `pattern`, newest first.
        fn glob(pattern: &str, path: &str) -> Vec<u8>;
        /// Search file contents under `path`. `mode` is `files`, `content` or `count`.
        fn grep(pattern: &str, path: &str, include: &str, mode: &str) -> Vec<u8>;
    }

    /// Commands, under the same root `fs` is bounded by.
    mod exec {
        /// Run a command through a shell, in `cwd` relative to the root.
        fn run(command: &str, cwd: &str, env: &[(&str, &str)]) -> Vec<u8>;
    }

    /// The other agents in this runtime.
    mod peers {
        /// Name them. The reply is an encoded `AgentList` carrying no configs.
        fn list() -> Vec<u8>;
    }

    /// The declaring agent's own past conversations.
    mod sessions {
        /// Search them. `request` is an encoded `SearchSessionsMsg`, the reply
        /// an encoded `SessionHitList`.
        fn search(request: &[u8]) -> Vec<u8>;
    }

    /// The skills the declaring agent named.
    mod skills {
        /// Name them. The reply is an encoded `SkillList`.
        fn list() -> Vec<u8>;
        /// One skill's instructions. The reply is an encoded `SkillBody`.
        fn get(name: &str) -> Vec<u8>;
    }

    /// Requests to the hosts a declaration named.
    mod http {
        /// Perform one request. The body stays bytes: a response is HTML or
        /// JSON far more often than it is UTF-8 anyone verified.
        fn fetch(
            method: &str,
            url: &str,
            body: &[u8],
            headers: &[(&str, &str)],
        ) -> (status: u16, headers: String, body: Vec<u8>);
    }
}
