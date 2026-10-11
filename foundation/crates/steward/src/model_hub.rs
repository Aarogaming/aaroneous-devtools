//! The model hub's pure boundary (lane C133; design in `governance/proposals/DEVTOOLS_MODEL_HUB.md`): does a model fit the
//! machine it is configured for, is an endpoint one we may talk to, is a config value a reference and not a secret, and what
//! did the model that answered actually say it was.
//!
//! Pure functions: no I/O, no clock, no environment. The program's entry point reads the secrets file and the host's real
//! figures and passes them in; nothing here ever sees a key.
//!
//! The memory figures are inputs, not constants: weights, key/value bytes per token and runtime overhead differ by model,
//! quantisation and server, so the config states them (measured on the host) and this module only does the arithmetic and
//! refuses what cannot work.

use crate::chain::digest;

const MB: u64 = 1024 * 1024;

/// One machine, in megabytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Host {
    pub name: String,
    pub vram_mb: u64,
    pub ram_mb: u64,
    /// Held back for the desktop, the display and everything else that is not the model (VRAM).
    pub reserved_vram_mb: u64,
    /// Held back for the operating system, the other seats and the build slot (RAM).
    pub reserved_ram_mb: u64,
}

/// One model as the config declares it, measured on the host or taken from the server's own report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub weights_mb: u64,
    /// Key/value cache bytes for one token of context (2 x layers x kv heads x head dim x bytes per value).
    pub kv_bytes_per_token: u64,
    pub context_tokens: u64,
    /// The server's own overhead (CUDA context, graphs, buffers).
    pub overhead_mb: u64,
}

impl Model {
    #[must_use]
    pub fn kv_mb(&self) -> u64 {
        (self.kv_bytes_per_token * self.context_tokens).div_ceil(MB)
    }

    /// Everything the model wants resident while it runs.
    #[must_use]
    pub fn needed_mb(&self) -> u64 {
        self.weights_mb + self.kv_mb() + self.overhead_mb
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fit {
    /// Fits in VRAM with room to spare.
    Gpu { needed_mb: u64, headroom_mb: u64 },
    /// Fits in VRAM, but with less than a tenth of it left: a longer prompt or a second job will not.
    Tight { needed_mb: u64, headroom_mb: u64 },
    /// Does not fit in VRAM; the server would have to spill `spill_mb` into system RAM, which is far slower.
    Offload { needed_mb: u64, spill_mb: u64 },
    /// Cannot run here at all.
    TooBig {
        needed_mb: u64,
        available_mb: u64,
        reason: String,
    },
}

impl Fit {
    /// True when the model runs fully on the GPU.
    #[must_use]
    pub fn on_gpu(&self) -> bool {
        matches!(self, Fit::Gpu { .. } | Fit::Tight { .. })
    }
}

/// Whether `model` can run on `host`. Offloading to RAM is a choice, not a default: without `allow_offload` a model that
/// does not fit in VRAM is `TooBig`. Offload also needs the spill to fit in the RAM left after the reservation, with a
/// quarter of that kept free.
#[must_use]
pub fn fit(host: &Host, model: &Model, allow_offload: bool) -> Fit {
    let needed = model.needed_mb();
    let vram = host.vram_mb.saturating_sub(host.reserved_vram_mb);
    if needed <= vram {
        let headroom = vram - needed;
        return if headroom * 10 < vram {
            Fit::Tight {
                needed_mb: needed,
                headroom_mb: headroom,
            }
        } else {
            Fit::Gpu {
                needed_mb: needed,
                headroom_mb: headroom,
            }
        };
    }
    let spill = needed - vram;
    let ram = host.ram_mb.saturating_sub(host.reserved_ram_mb);
    if !allow_offload {
        return Fit::TooBig {
            needed_mb: needed,
            available_mb: vram,
            reason: format!(
                "needs {needed} MB, {} has {vram} MB of VRAM free of its reservation and offload is off",
                host.name
            ),
        };
    }
    if spill * 4 > ram * 3 {
        return Fit::TooBig {
            needed_mb: needed,
            available_mb: vram + ram * 3 / 4,
            reason: format!(
                "needs {spill} MB beyond VRAM, more than the {} MB of RAM it may use",
                ram * 3 / 4
            ),
        };
    }
    Fit::Offload {
        needed_mb: needed,
        spill_mb: spill,
    }
}

/// The largest candidate that runs fully on the GPU; ties by name so the same inputs always choose the same model.
/// `None` when none does: the caller then falls back to a smaller class or a remote provider, never to an offloaded giant.
#[must_use]
pub fn choose<'a>(host: &Host, candidates: &'a [Model]) -> Option<&'a Model> {
    candidates
        .iter()
        .filter(|m| fit(host, m, false).on_gpu())
        .max_by(|a, b| {
            a.weights_mb
                .cmp(&b.weights_mb)
                .then_with(|| b.name.cmp(&a.name))
        })
}

/// Which hosts a model endpoint may name. Loopback is always allowed; anything else must be listed, and plain `http` off
/// loopback must be listed separately (an encrypted overlay or a trusted segment), so a typo cannot send prompts and keys
/// in the clear to an arbitrary address.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointPolicy {
    pub allowed_hosts: Vec<String>,
    pub plain_http_hosts: Vec<String>,
}

/// Why `url` may not be used as a model endpoint, or `None`.
#[must_use]
pub fn endpoint_problem(url: &str, policy: &EndpointPolicy) -> Option<String> {
    if url.is_empty() || url.len() > 300 || url.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Some(
            "endpoint must be a short URL with no spaces or control characters".to_owned(),
        );
    }
    let Some((scheme, rest)) = url.split_once("://") else {
        return Some("endpoint must start with http:// or https://".to_owned());
    };
    if scheme != "http" && scheme != "https" {
        return Some(format!("endpoint scheme {scheme:?} must be http or https"));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Some("endpoint must not embed credentials (user@host)".to_owned());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if !authority.starts_with('[') || h.ends_with(']') => {
            (h.trim_matches(['[', ']']), Some(p))
        }
        _ => (authority.trim_matches(['[', ']']), None),
    };
    if host.is_empty() {
        return Some("endpoint has no host".to_owned());
    }
    if let Some(port) = port {
        if !port.chars().all(|c| c.is_ascii_digit())
            || port.parse::<u32>().map_or(true, |p| p == 0 || p > 65535)
        {
            return Some(format!("endpoint port {port:?} must be 1-65535"));
        }
    }
    let lowered = host.to_ascii_lowercase();
    let loopback = lowered == "localhost"
        || lowered == "::1"
        || lowered.starts_with("127.") && lowered.split('.').count() == 4;
    if loopback {
        return None;
    }
    if !policy
        .allowed_hosts
        .iter()
        .any(|h| h.eq_ignore_ascii_case(host))
    {
        return Some(format!(
            "endpoint host {host:?} is not loopback and is not in the allowed hosts"
        ));
    }
    if scheme == "http"
        && !policy
            .plain_http_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
    {
        return Some(format!(
            "endpoint {host:?} is reachable only over plain http; use https or list it as a trusted plain-http host"
        ));
    }
    None
}

fn is_ref(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some('a'..='z'))
        && text.len() <= 48
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
}

/// Why a config field's value is not acceptable, or `None`. A field whose name suggests a secret (`key`, `token`,
/// `secret`, `password`) must hold a *reference name* that the entry point resolves from the secrets file, never the
/// secret; a value shaped like a key is refused wherever it appears.
#[must_use]
pub fn config_value_problem(field: &str, value: &str) -> Option<String> {
    let name = field.to_ascii_lowercase();
    let secret_field = ["key", "token", "secret", "password"]
        .iter()
        .any(|w| name.contains(w));
    if secret_field && !is_ref(value) {
        return Some(format!(
            "{field} must be a reference name such as \"provider-openai\", not a secret value"
        ));
    }
    if looks_like_key(value) {
        return Some(format!(
            "{field} holds something shaped like a credential; keep it in the secrets file and refer to it by name"
        ));
    }
    None
}

fn looks_like_key(value: &str) -> bool {
    let v = value.trim();
    if v.starts_with("sk-")
        || v.starts_with("ghp_")
        || v.starts_with("gho_")
        || v.starts_with("github_pat_")
        || v.starts_with("AIza")
        || v.to_ascii_lowercase().starts_with("bearer ")
    {
        return true;
    }
    let run = v
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '/' | '='))
        .count();
    v.len() >= 32
        && run == v.chars().count()
        && v.chars().any(|c| c.is_ascii_digit())
        && v.chars().any(|c| c.is_ascii_alphabetic())
        && !v.contains(['/', '.'])
        && !v.contains("--")
}

/// What the hub records for one model call: which model was asked for, which one the server said answered, and digests of
/// the request and the response. The digests prove what was exchanged without keeping the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub node: String,
    pub provider: String,
    pub model_requested: String,
    pub model_reported: String,
    pub request_digest: String,
    pub response_digest: String,
    pub tick: u64,
}

impl Receipt {
    /// Identity of the receipt: any field changing changes it.
    #[must_use]
    pub fn id(&self) -> String {
        digest(&format!(
            "receipt-v1\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            self.node,
            self.provider,
            self.model_requested,
            self.model_reported,
            self.request_digest,
            self.response_digest,
            self.tick
        ))
    }

    /// Set when the server answered with a different model than the one asked for (a silent fallback, a mislabelled
    /// alias). The independence rule counts the model that ran, so this is recorded, never smoothed over.
    #[must_use]
    pub fn mismatch(&self) -> Option<String> {
        (self.model_requested != self.model_reported).then(|| {
            format!(
                "asked for {:?} but {:?} answered ({}): the receipt records the model that ran",
                self.model_requested, self.model_reported, self.provider
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host_4060() -> Host {
        Host {
            name: "aaroneoushome".into(),
            vram_mb: 8192,
            ram_mb: 16384,
            reserved_vram_mb: 600,
            reserved_ram_mb: 4096,
        }
    }

    /// Qwen2.5-Coder-14B-Instruct AWQ, 8k context: published architecture is 48 layers, 8 kv heads, head dim 128, fp16 cache
    /// (2 x 48 x 8 x 128 x 2 = 196,608 bytes per token); the AWQ files are about 9.4 GB. Estimates, not measurements.
    fn qwen_14b_awq() -> Model {
        Model {
            name: "qwen2.5-coder-14b-awq".into(),
            weights_mb: 9400,
            kv_bytes_per_token: 196_608,
            context_tokens: 8192,
            overhead_mb: 1000,
        }
    }

    /// A 7B-class Q4 GGUF: 28 layers, 4 kv heads, head dim 128 gives 57,344 bytes per token; the file is about 4.7 GB.
    fn qwen_7b_q4() -> Model {
        Model {
            name: "qwen2.5-coder-7b-q4".into(),
            weights_mb: 4700,
            kv_bytes_per_token: 57_344,
            context_tokens: 8192,
            overhead_mb: 500,
        }
    }

    #[test]
    fn kv_cache_is_bytes_per_token_times_context() {
        assert_eq!(qwen_14b_awq().kv_mb(), 1536);
        assert_eq!(qwen_7b_q4().kv_mb(), 448);
        assert_eq!(qwen_7b_q4().needed_mb(), 4700 + 448 + 500);
    }

    #[test]
    fn a_14b_awq_does_not_fit_an_8gb_4060_and_says_so() {
        let result = fit(&host_4060(), &qwen_14b_awq(), false);
        assert!(matches!(result, Fit::TooBig { .. }), "{result:?}");
        assert!(!result.on_gpu());
    }

    #[test]
    fn offloading_is_a_choice_and_is_reported_as_slow_not_as_a_fit() {
        let result = fit(&host_4060(), &qwen_14b_awq(), true);
        match result {
            Fit::Offload { spill_mb, .. } => {
                assert_eq!(spill_mb, 9400 + 1536 + 1000 - (8192 - 600))
            }
            other => panic!("{other:?}"),
        }
        assert!(!result.on_gpu());
    }

    #[test]
    fn offload_that_would_exhaust_ram_is_refused() {
        let small_ram = Host {
            ram_mb: 8192,
            ..host_4060()
        };
        assert!(matches!(
            fit(&small_ram, &qwen_14b_awq(), true),
            Fit::TooBig { .. }
        ));
    }

    #[test]
    fn a_7b_q4_fits_with_room() {
        let result = fit(&host_4060(), &qwen_7b_q4(), false);
        assert!(matches!(result, Fit::Gpu { .. }), "{result:?}");
    }

    #[test]
    fn a_model_with_under_a_tenth_left_is_tight() {
        let nearly = Model {
            weights_mb: 6300,
            ..qwen_7b_q4()
        };
        assert!(matches!(
            fit(&host_4060(), &nearly, false),
            Fit::Tight { .. }
        ));
        assert!(fit(&host_4060(), &nearly, false).on_gpu());
    }

    #[test]
    fn the_hub_picks_the_largest_model_that_runs_on_the_gpu_and_never_an_offloaded_one() {
        let candidates = [
            qwen_14b_awq(),
            qwen_7b_q4(),
            Model {
                name: "tiny".into(),
                weights_mb: 1200,
                ..qwen_7b_q4()
            },
        ];
        assert_eq!(
            choose(&host_4060(), &candidates).map(|m| m.name.as_str()),
            Some("qwen2.5-coder-7b-q4")
        );
        let none_fit = [qwen_14b_awq()];
        assert_eq!(choose(&host_4060(), &none_fit), None);
        let big_host = Host {
            vram_mb: 24576,
            ..host_4060()
        };
        assert_eq!(
            choose(&big_host, &candidates).map(|m| m.name.as_str()),
            Some("qwen2.5-coder-14b-awq")
        );
    }

    #[test]
    fn choice_is_deterministic_when_sizes_tie() {
        let a = Model {
            name: "a".into(),
            ..qwen_7b_q4()
        };
        let b = Model {
            name: "b".into(),
            ..qwen_7b_q4()
        };
        assert_eq!(
            choose(&host_4060(), &[b.clone(), a.clone()]).map(|m| m.name.clone()),
            Some("a".into())
        );
        assert_eq!(
            choose(&host_4060(), &[a, b]).map(|m| m.name.clone()),
            Some("a".into())
        );
    }

    #[test]
    fn a_reservation_larger_than_the_card_leaves_nothing_not_an_underflow() {
        let starved = Host {
            reserved_vram_mb: 99_999,
            ..host_4060()
        };
        assert!(matches!(
            fit(&starved, &qwen_7b_q4(), false),
            Fit::TooBig { .. }
        ));
    }

    fn policy() -> EndpointPolicy {
        EndpointPolicy {
            allowed_hosts: vec!["100.109.8.12".into()],
            plain_http_hosts: vec![],
        }
    }

    #[test]
    fn loopback_endpoints_are_allowed_and_everything_else_must_be_listed() {
        for ok in [
            "http://localhost:8000/v1",
            "http://127.0.0.1:8000/v1",
            "http://[::1]:11434",
            "https://100.109.8.12:8443/v1",
        ] {
            assert_eq!(endpoint_problem(ok, &policy()), None, "{ok}");
        }
        for bad in [
            "http://203.0.113.9:8000/v1",
            "https://203.0.113.9:8443/v1",
            "https://evil.example/v1",
            "http://100.109.8.12:8000/v1",
            "ftp://localhost/",
            "localhost:8000",
            "http://user:pass@localhost:8000/",
            "http://localhost:0/",
            "http://localhost:99999/",
            "http://localhost:80a/",
            "http://",
            "http://local host/",
            "http://127.0.0.1.evil.example/v1",
        ] {
            assert!(endpoint_problem(bad, &policy()).is_some(), "{bad}");
        }
    }

    #[test]
    fn plain_http_off_loopback_needs_its_own_listing() {
        let trusted = EndpointPolicy {
            plain_http_hosts: vec!["100.109.8.12".into()],
            ..policy()
        };
        assert_eq!(
            endpoint_problem("http://100.109.8.12:8000/v1", &trusted),
            None
        );
    }

    #[test]
    fn secret_fields_hold_a_reference_never_the_value() {
        assert_eq!(config_value_problem("api_key_ref", "provider-openai"), None);
        assert_eq!(
            config_value_problem("endpoint", "http://localhost:8000/v1"),
            None
        );
        assert_eq!(
            config_value_problem("model", "qwen2.5-coder-7b-instruct-q4_k_m"),
            None
        );
        for (field, value) in [
            ("api_key", "sk-proj-abcdefghijklmnopqrstuvwxyz0123456789"),
            ("token", "ghp_abcdefghijklmnopqrstuvwxyz0123456789"),
            ("api_key", "Provider OpenAI"),
            ("password", ""),
            ("notes", "Bearer abcdefghijklmnop"),
            ("anything", "AIzaSyA-abcdefghijklmnopqrstuvwxyz012345"),
            ("anything", "aB3dE5gH7jK9mN1pQ3sT5vW7yZ9bC1dF3gH5"),
        ] {
            assert!(
                config_value_problem(field, value).is_some(),
                "{field}={value}"
            );
        }
    }

    fn receipt() -> Receipt {
        Receipt {
            node: "ubuntu-1".into(),
            provider: "local-vllm".into(),
            model_requested: "qwen2.5-coder-7b".into(),
            model_reported: "qwen2.5-coder-7b".into(),
            request_digest: "a".repeat(64),
            response_digest: "b".repeat(64),
            tick: 7,
        }
    }

    #[test]
    fn a_receipt_id_changes_with_every_field_and_a_substituted_model_is_flagged() {
        let base = receipt();
        assert_eq!(base.id(), receipt().id());
        for variant in [
            Receipt {
                node: "x".into(),
                ..receipt()
            },
            Receipt {
                provider: "x".into(),
                ..receipt()
            },
            Receipt {
                model_requested: "x".into(),
                ..receipt()
            },
            Receipt {
                model_reported: "x".into(),
                ..receipt()
            },
            Receipt {
                request_digest: "c".repeat(64),
                ..receipt()
            },
            Receipt {
                response_digest: "c".repeat(64),
                ..receipt()
            },
            Receipt {
                tick: 8,
                ..receipt()
            },
        ] {
            assert_ne!(base.id(), variant.id());
        }
        assert_eq!(base.mismatch(), None);
        let swapped = Receipt {
            model_reported: "qwen3.5-9b".into(),
            ..receipt()
        };
        assert!(swapped.mismatch().is_some_and(|m| m.contains("answered")));
    }
}
