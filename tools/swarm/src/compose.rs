use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    fs::File,
    io::Write,
    num::NonZeroU16,
    path::PathBuf,
};

use color_eyre::eyre::{eyre, Context, ContextCompat};
use iroha_crypto::{Algorithm, ExposedPrivateKey, KeyPair, PrivateKey, PublicKey};
use iroha_data_model::{prelude::PeerId, ChainId};
use iroha_primitives::addr::{socket_addr, SocketAddr};
use peer_generator::Peer;
use serde::{ser::SerializeMap, Serialize, Serializer};

use crate::{cli::SourceParsed, util::AbsolutePath};

/// Config directory inside of the docker image
const DIR_CONFIG_IN_DOCKER: &str = "/config";
const GENESIS_KEYPAIR_SEED: &[u8; 7] = b"genesis";
const GENESIS_SIGNED_FILE: &str = "/tmp/genesis.signed.scale";
const COMMAND_SIGN_AND_SUBMIT_GENESIS: &str = r#"/bin/sh -c "
kagami genesis sign /config/genesis.json --public-key $$GENESIS_PUBLIC_KEY --private-key $$GENESIS_PRIVATE_KEY --out-file $$GENESIS_SIGNED_FILE &&
irohad --submit-genesis
""#;
const DOCKER_COMPOSE_VERSION: &str = "3.8";
const PLATFORM_ARCHITECTURE: &str = "linux/amd64";

#[derive(Serialize, Debug)]
pub struct DockerCompose {
    version: DockerComposeVersion,
    services: BTreeMap<String, DockerComposeService>,
}

impl DockerCompose {
    pub fn new(services: BTreeMap<String, DockerComposeService>) -> Self {
        Self {
            version: DockerComposeVersion,
            services,
        }
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        banner_enabled: bool,
    ) -> Result<(), color_eyre::Report> {
        let mut file = File::create(path)
            .wrap_err_with(|| eyre!("Failed to create file {}", path.display()))?;

        if banner_enabled {
            file.write_all(
                b"# This file is generated by iroha_swarm.\n\
                  # Do not edit it manually.\n\n",
            )
            .wrap_err_with(|| eyre!("Failed to write banner into {}", path.display()))?;
        }

        let yaml = serde_yaml::to_string(self).wrap_err("Failed to serialise YAML")?;
        file.write_all(yaml.as_bytes())
            .wrap_err_with(|| eyre!("Failed to write YAML content into {}", path.display()))
            .map_err(Into::into)
    }
}

#[derive(Debug)]
struct DockerComposeVersion;

impl Serialize for DockerComposeVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(DOCKER_COMPOSE_VERSION)
    }
}

#[derive(Debug)]
struct PlatformArchitecture;

impl Serialize for PlatformArchitecture {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(PLATFORM_ARCHITECTURE)
    }
}

pub struct DockerComposeServiceBuilder {
    chain_id: ChainId,
    peer: Peer,
    source: ServiceSource,
    volumes: Vec<(String, String)>,
    trusted_peers: BTreeSet<PeerId>,
    genesis_public_key: PublicKey,
    genesis_private_key: Option<PrivateKey>,
    health_check: bool,
}

#[derive(Serialize, Debug)]
pub struct DockerComposeService {
    #[serde(flatten)]
    source: ServiceSource,
    platform: PlatformArchitecture,
    environment: FullPeerEnv,
    ports: Vec<PairColon<u16, u16>>,
    volumes: Vec<PairColon<String, String>>,
    init: AlwaysTrue,
    #[serde(skip_serializing_if = "ServiceCommand::is_none")]
    command: ServiceCommand,
    #[serde(skip_serializing_if = "Option::is_none")]
    healthcheck: Option<HealthCheck>,
}

impl DockerComposeServiceBuilder {
    pub fn new(
        chain_id: ChainId,
        peer: Peer,
        source: ServiceSource,
        volumes: Vec<(String, String)>,
        trusted_peers: BTreeSet<PeerId>,
        genesis_public_key: PublicKey,
    ) -> Self {
        Self {
            chain_id,
            peer,
            source,
            volumes,
            trusted_peers,
            genesis_public_key,
            genesis_private_key: None,
            health_check: false,
        }
    }

    pub fn set_health_check(mut self, flag: bool) -> Self {
        self.health_check = flag;
        self
    }

    pub fn submit_genesis_with(mut self, private_key: PrivateKey) -> Self {
        self.genesis_private_key = Some(private_key);
        self
    }

    pub fn build(self) -> DockerComposeService {
        let Self {
            chain_id,
            peer,
            source,
            volumes,
            trusted_peers,
            genesis_public_key,
            genesis_private_key,
            health_check,
        } = self;

        let ports = vec![
            PairColon(peer.port_p2p, peer.port_p2p),
            PairColon(peer.port_api, peer.port_api),
        ];

        let genesis_signed_file = genesis_private_key
            .as_ref()
            .map(|_| GENESIS_SIGNED_FILE.to_owned());
        let command = genesis_private_key.map_or(ServiceCommand::None, |genesis_private_key| {
            ServiceCommand::SignAndSubmitGenesis {
                genesis_private_key,
            }
        });

        let compact_env = CompactPeerEnv {
            chain_id,
            trusted_peers,
            genesis_public_key,
            genesis_signed_file,
            key_pair: peer.key_pair.clone(),
            p2p_addr: socket_addr!(0.0.0.0:peer.port_p2p),
            api_addr: socket_addr!(0.0.0.0:peer.port_api),
        };

        DockerComposeService {
            source,
            platform: PlatformArchitecture,
            command,
            init: AlwaysTrue,
            volumes: volumes.into_iter().map(|(a, b)| PairColon(a, b)).collect(),
            ports,
            environment: compact_env.into(),
            healthcheck: health_check.then_some(HealthCheck {
                port: peer.port_api,
            }),
        }
    }
}

#[derive(Debug)]
struct AlwaysTrue;

impl Serialize for AlwaysTrue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(true)
    }
}

#[derive(Debug)]
enum ServiceCommand {
    SignAndSubmitGenesis { genesis_private_key: PrivateKey },
    None,
}

impl ServiceCommand {
    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Serialize for ServiceCommand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::None => serializer.serialize_none(),
            Self::SignAndSubmitGenesis {
                genesis_private_key,
            } => {
                let genesis_private_key =
                    ExposedPrivateKey(genesis_private_key.clone()).to_string();
                let command = COMMAND_SIGN_AND_SUBMIT_GENESIS
                    .replace("$$GENESIS_PRIVATE_KEY", &genesis_private_key);
                serializer.serialize_str(&command)
            }
        }
    }
}

/// Serializes as a Iroha health check according to the
/// [spec](https://docs.docker.com/compose/compose-file/compose-file-v3/#healthcheck).
#[derive(Debug)]
struct HealthCheck {
    #[allow(dead_code)]
    port: u16,
}

const HEALTH_CHECK_INTERVAL: &str = "2s"; // half of default pipeline time

const HEALTH_CHECK_TIMEOUT: &str = "1s"; // status request usually resolves immediately

const HEALTH_CHECK_RETRIES: u8 = 30u8; // try within one minute given the interval

const HEALTH_CHECK_START_PERIOD: &str = "4s"; // default pipeline time

impl Serialize for HealthCheck {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry(
            "test",
            &format!(
                "test $(curl -s http://127.0.0.1:{}/status/blocks) -gt 0",
                self.port
            ),
        )?;
        map.serialize_entry("interval", HEALTH_CHECK_INTERVAL)?;
        map.serialize_entry("timeout", HEALTH_CHECK_TIMEOUT)?;
        map.serialize_entry("retries", &HEALTH_CHECK_RETRIES)?;
        map.serialize_entry("start_period", HEALTH_CHECK_START_PERIOD)?;
        map.end()
    }
}

/// Serializes as `"{0}:{1}"`
#[derive(derive_more::Display, Debug)]
#[display(fmt = "{_0}:{_1}")]
struct PairColon<T, U>(T, U)
where
    T: Display,
    U: Display;

impl<T, U> Serialize for PairColon<T, U>
where
    T: Display,
    U: Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ServiceSource {
    Image(String),
    Build(PathBuf),
}

#[serde_with::serde_as]
#[serde_with::skip_serializing_none]
#[derive(Serialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
struct FullPeerEnv {
    chain_id: ChainId,
    public_key: PublicKey,
    private_key: ExposedPrivateKey,
    p2p_address: SocketAddr,
    api_address: SocketAddr,
    genesis_public_key: PublicKey,
    genesis_signed_file: Option<String>,
    #[serde_as(as = "Option<serde_with::json::JsonString>")]
    sumeragi_trusted_peers: Option<BTreeSet<PeerId>>,
}

struct CompactPeerEnv {
    chain_id: ChainId,
    key_pair: KeyPair,
    genesis_public_key: PublicKey,
    genesis_signed_file: Option<String>,
    p2p_addr: SocketAddr,
    api_addr: SocketAddr,
    trusted_peers: BTreeSet<PeerId>,
}

impl From<CompactPeerEnv> for FullPeerEnv {
    fn from(value: CompactPeerEnv) -> Self {
        Self {
            chain_id: value.chain_id,
            public_key: value.key_pair.public_key().clone(),
            private_key: ExposedPrivateKey(value.key_pair.private_key().clone()),
            genesis_public_key: value.genesis_public_key,
            genesis_signed_file: value.genesis_signed_file,
            p2p_address: value.p2p_addr,
            api_address: value.api_addr,
            sumeragi_trusted_peers: if value.trusted_peers.is_empty() {
                None
            } else {
                Some(value.trusted_peers)
            },
        }
    }
}

#[derive(Debug)]
pub struct DockerComposeBuilder<'a> {
    /// Needed to compute a relative source build path
    pub target_file: &'a AbsolutePath,
    /// Needed to put into `volumes`
    pub config_dir: &'a AbsolutePath,
    pub image_source: ResolvedImageSource,
    pub peers: NonZeroU16,
    /// Crypto seed to use for keys generation
    pub seed: Option<&'a [u8]>,
    pub health_check: bool,
}

impl DockerComposeBuilder<'_> {
    fn build(&self) -> color_eyre::Result<DockerCompose> {
        let target_file_dir = self.target_file.parent().ok_or_else(|| {
            eyre!(
                "Cannot get a directory of a file {}",
                self.target_file.display()
            )
        })?;

        let chain_id = ChainId::from("00000000-0000-0000-0000-000000000000");
        let peers = peer_generator::generate_peers(self.peers, self.seed)
            .wrap_err("Failed to generate peers")?;
        let genesis_key_pair = generate_key_pair(self.seed, GENESIS_KEYPAIR_SEED);
        let service_source = match &self.image_source {
            ResolvedImageSource::Build { path } => {
                ServiceSource::Build(path.relative_to(target_file_dir)?)
            }
            ResolvedImageSource::Image { name } => ServiceSource::Image(name.clone()),
        };
        let volumes = vec![(
            self.config_dir
                .relative_to(target_file_dir)?
                .to_str()
                .wrap_err("Config directory path is not a valid string")?
                .to_owned(),
            DIR_CONFIG_IN_DOCKER.to_owned(),
        )];

        let trusted_peers: BTreeSet<PeerId> = peers.values().map(Peer::id_as_a_service).collect();

        let mut peers_iter = peers.iter();

        let first_peer_service = {
            let (name, peer) = peers_iter.next().expect("There is non-zero count of peers");
            let service = DockerComposeServiceBuilder::new(
                chain_id.clone(),
                peer.clone(),
                service_source.clone(),
                volumes.clone(),
                trusted_peers
                    .iter()
                    .filter(|trusted_peer| trusted_peer.public_key() != peer.key_pair.public_key())
                    .cloned()
                    .collect(),
                genesis_key_pair.public_key().clone(),
            )
            .submit_genesis_with(genesis_key_pair.private_key().clone())
            .set_health_check(self.health_check)
            .build();

            (name.clone(), service)
        };

        let services = peers_iter
            .map(|(name, peer)| {
                let service = DockerComposeServiceBuilder::new(
                    chain_id.clone(),
                    peer.clone(),
                    service_source.clone(),
                    volumes.clone(),
                    trusted_peers
                        .iter()
                        .filter(|trusted_peer| {
                            trusted_peer.public_key() != peer.key_pair.public_key()
                        })
                        .cloned()
                        .collect(),
                    genesis_key_pair.public_key().clone(),
                )
                .set_health_check(self.health_check)
                .build();

                (name.clone(), service)
            })
            .chain(std::iter::once(first_peer_service))
            .collect();

        let compose = DockerCompose::new(services);
        Ok(compose)
    }

    pub(crate) fn build_and_write(&self, banner_enabled: bool) -> color_eyre::Result<()> {
        let target_file = self.target_file;
        let compose = self
            .build()
            .wrap_err("Failed to build a docker compose file")?;
        compose.write_file(&target_file.path, banner_enabled)
    }
}

fn generate_key_pair(base_seed: Option<&[u8]>, additional_seed: &[u8]) -> KeyPair {
    base_seed.map_or_else(KeyPair::random, |base| {
        let seed: Vec<_> = base.iter().chain(additional_seed).copied().collect();
        KeyPair::from_seed(seed, Algorithm::default())
    })
}

mod peer_generator {
    use std::{collections::BTreeMap, num::NonZeroU16};

    use color_eyre::Report;
    use iroha_crypto::KeyPair;
    use iroha_data_model::prelude::PeerId;
    use iroha_primitives::addr::{SocketAddr, SocketAddrHost};

    const BASE_PORT_P2P: u16 = 1337;
    const BASE_PORT_API: u16 = 8080;
    const BASE_SERVICE_NAME: &'_ str = "irohad";

    #[derive(Clone)]
    pub struct Peer {
        pub name: String,
        pub port_p2p: u16,
        pub port_api: u16,
        pub key_pair: KeyPair,
    }

    impl Peer {
        /// [`PeerId`] with an address containing service name as a host, therefore reachable
        /// from other Docker Compose services.
        pub fn id_as_a_service(&self) -> PeerId {
            let address = SocketAddr::Host(SocketAddrHost {
                host: self.name.clone().into(),
                port: self.port_p2p,
            });

            PeerId::new(address.clone(), self.key_pair.public_key().clone())
        }
    }

    pub fn generate_peers(
        peers: NonZeroU16,
        base_seed: Option<&[u8]>,
    ) -> Result<BTreeMap<String, Peer>, Report> {
        (0u16..peers.get())
            .map(|i| {
                let service_name = format!("{BASE_SERVICE_NAME}{i}");

                let key_pair = super::generate_key_pair(base_seed, service_name.as_bytes());

                let peer = Peer {
                    name: service_name.clone(),
                    port_p2p: BASE_PORT_P2P + i,
                    port_api: BASE_PORT_API + i,
                    key_pair,
                };

                Ok((service_name, peer))
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum ResolvedImageSource {
    Image { name: String },
    Build { path: AbsolutePath },
}

impl TryFrom<SourceParsed> for ResolvedImageSource {
    type Error = color_eyre::Report;

    fn try_from(value: SourceParsed) -> Result<Self, Self::Error> {
        let resolved = match value {
            SourceParsed::Image { name } => Self::Image { name },
            SourceParsed::Build { path: relative } => {
                let absolute =
                    AbsolutePath::absolutize(&relative).wrap_err("Failed to resolve build path")?;
                Self::Build { path: absolute }
            }
        };

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet, HashMap, HashSet},
        path::{Path, PathBuf},
        str::FromStr,
    };

    use iroha_config::{
        base::{env::MockEnv, read::ConfigReader},
        parameters::user::Root as UserConfig,
    };
    use iroha_crypto::KeyPair;
    use iroha_primitives::addr::{socket_addr, SocketAddr};
    use path_absolutize::Absolutize;

    use super::*;

    impl AbsolutePath {
        pub(crate) fn from_virtual(path: &PathBuf, virtual_root: impl AsRef<Path> + Sized) -> Self {
            let path = path
                .absolutize_virtually(virtual_root)
                .unwrap()
                .to_path_buf();
            Self { path }
        }
    }

    impl From<FullPeerEnv> for MockEnv {
        fn from(peer_env: FullPeerEnv) -> Self {
            let json = serde_json::to_string(&peer_env).expect("Must be serializable");
            let env: HashMap<_, String> =
                serde_json::from_str(&json).expect("Must be deserializable into a hash map");
            Self::with_map(env)
        }
    }

    impl From<CompactPeerEnv> for MockEnv {
        fn from(value: CompactPeerEnv) -> Self {
            let full: FullPeerEnv = value.into();
            full.into()
        }
    }

    #[test]
    fn default_config_with_swarm_env_is_exhaustive() {
        let keypair = KeyPair::random();
        let env: MockEnv = CompactPeerEnv {
            chain_id: ChainId::from("00000000-0000-0000-0000-000000000000"),
            key_pair: keypair.clone(),
            genesis_public_key: keypair.public_key().clone(),
            genesis_signed_file: Some("/tmp/genesis.signed.scale".to_owned()),
            p2p_addr: socket_addr!(127.0.0.1:1337),
            api_addr: socket_addr!(127.0.0.1:1338),
            trusted_peers: {
                let mut set = BTreeSet::new();
                set.insert(PeerId::new(
                    socket_addr!(127.0.0.1:8081),
                    KeyPair::random().into_parts().0,
                ));
                set
            },
        }
        .into();

        let _ = ConfigReader::new()
            .with_env(env.clone())
            .read_and_complete::<UserConfig>()
            .expect("config in env should be exhaustive");

        assert_eq!(env.unvisited(), HashSet::new());
    }

    #[test]
    fn serialize_image_source() {
        let source = ServiceSource::Image("hyperledger/iroha2:stable".to_owned());
        let serialised = serde_json::to_string(&source).unwrap();
        assert_eq!(serialised, r#"{"image":"hyperledger/iroha2:stable"}"#);
    }

    #[test]
    fn serialize_docker_compose() {
        let compose = DockerCompose {
            version: DockerComposeVersion,
            services: {
                let mut map = BTreeMap::new();

                let chain_id = ChainId::from("00000000-0000-0000-0000-000000000000");
                let key_pair =
                    KeyPair::from_seed(vec![1, 5, 1, 2, 2, 3, 4, 1, 2, 3], Algorithm::default());

                map.insert(
                    "iroha0".to_owned(),
                    DockerComposeService {
                        platform: PlatformArchitecture,
                        source: ServiceSource::Build(PathBuf::from(".")),
                        environment: CompactPeerEnv {
                            chain_id,
                            key_pair: key_pair.clone(),
                            genesis_public_key: key_pair.public_key().clone(),
                            genesis_signed_file: Some("/tmp/genesis.signed.scale".to_owned()),
                            p2p_addr: SocketAddr::from_str("iroha1:1339").unwrap(),
                            api_addr: SocketAddr::from_str("iroha1:1338").unwrap(),
                            trusted_peers: BTreeSet::new(),
                        }
                        .into(),
                        ports: vec![
                            PairColon(1337, 1337),
                            PairColon(8080, 8080),
                            PairColon(8081, 8081),
                        ],
                        volumes: vec![PairColon(
                            "./configs/peer/legacy_stable".to_owned(),
                            "/config".to_owned(),
                        )],
                        init: AlwaysTrue,
                        command: ServiceCommand::SignAndSubmitGenesis {
                            genesis_private_key: key_pair.private_key().clone(),
                        },
                        healthcheck: None,
                    },
                );

                map
            },
        };

        let actual = serde_yaml::to_string(&compose).expect("Should be serialisable");
        #[allow(clippy::needless_raw_string_hashes)]
        let expected = expect_test::expect![[r#"
            version: '3.8'
            services:
              iroha0:
                build: .
                platform: linux/amd64
                environment:
                  CHAIN_ID: 00000000-0000-0000-0000-000000000000
                  PUBLIC_KEY: ed012039E5BF092186FACC358770792A493CA98A83740643A3D41389483CF334F748C8
                  PRIVATE_KEY: 802640DB9D90D20F969177BD5882F9FE211D14D1399D5440D04E3468783D169BBC4A8E39E5BF092186FACC358770792A493CA98A83740643A3D41389483CF334F748C8
                  P2P_ADDRESS: iroha1:1339
                  API_ADDRESS: iroha1:1338
                  GENESIS_PUBLIC_KEY: ed012039E5BF092186FACC358770792A493CA98A83740643A3D41389483CF334F748C8
                  GENESIS_SIGNED_FILE: /tmp/genesis.signed.scale
                ports:
                - 1337:1337
                - 8080:8080
                - 8081:8081
                volumes:
                - ./configs/peer/legacy_stable:/config
                init: true
                command: |-
                  /bin/sh -c "
                  kagami genesis sign /config/genesis.json --public-key $$GENESIS_PUBLIC_KEY --private-key 802640DB9D90D20F969177BD5882F9FE211D14D1399D5440D04E3468783D169BBC4A8E39E5BF092186FACC358770792A493CA98A83740643A3D41389483CF334F748C8 --out-file $$GENESIS_SIGNED_FILE &&
                  irohad --submit-genesis
                  "
        "#]];
        expected.assert_eq(&actual);
    }

    #[test]
    fn empty_genesis_private_key_is_skipped_in_env() {
        let chain_id = ChainId::from("00000000-0000-0000-0000-000000000000");

        let key_pair = KeyPair::from_seed(vec![0, 1, 2], Algorithm::default());

        let env: FullPeerEnv = CompactPeerEnv {
            chain_id,
            key_pair: key_pair.clone(),
            genesis_public_key: key_pair.public_key().clone(),
            genesis_signed_file: None,
            p2p_addr: SocketAddr::from_str("iroha0:1337").unwrap(),
            api_addr: SocketAddr::from_str("iroha0:1337").unwrap(),
            trusted_peers: BTreeSet::new(),
        }
        .into();

        let actual = serde_yaml::to_string(&env).unwrap();
        #[allow(clippy::needless_raw_string_hashes)]
        let expected = expect_test::expect![[r#"
            CHAIN_ID: 00000000-0000-0000-0000-000000000000
            PUBLIC_KEY: ed0120415388A90FA238196737746A70565D041CFB32EAA0C89FF8CB244C7F832A6EBD
            PRIVATE_KEY: 8026406BF163FD75192B81A78CB20C5F8CB917F591AC6635F2577E6CA305C27A456A5D415388A90FA238196737746A70565D041CFB32EAA0C89FF8CB244C7F832A6EBD
            P2P_ADDRESS: iroha0:1337
            API_ADDRESS: iroha0:1337
            GENESIS_PUBLIC_KEY: ed0120415388A90FA238196737746A70565D041CFB32EAA0C89FF8CB244C7F832A6EBD
        "#]];
        expected.assert_eq(&actual);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn generate_peers_deterministically() {
        let root = Path::new("/");
        let seed = Some(b"iroha".to_vec());
        let seed = seed.as_deref();

        let composed = DockerComposeBuilder {
            target_file: &AbsolutePath::from_virtual(
                &PathBuf::from("/test/docker-compose.yml"),
                root,
            ),
            config_dir: &AbsolutePath::from_virtual(&PathBuf::from("/test/config"), root),
            peers: 4.try_into().unwrap(),
            image_source: ResolvedImageSource::Build {
                path: AbsolutePath::from_virtual(&PathBuf::from("/test/iroha-cloned"), root),
            },
            seed,
            health_check: true,
        }
        .build()
        .expect("should build with no errors");

        let yaml = serde_yaml::to_string(&composed).unwrap();
        let expected = expect_test::expect![[r#"
            version: '3.8'
            services:
              irohad0:
                build: ./iroha-cloned
                platform: linux/amd64
                environment:
                  CHAIN_ID: 00000000-0000-0000-0000-000000000000
                  PUBLIC_KEY: ed0120AB0B22BC053C954A4CA7CF451872E9C5B971F0DA5D92133648226D02E3ABB611
                  PRIVATE_KEY: 80264078DEFA845766A579C9F84CE8840864615B2913073E1321930DD087F77017F1A4AB0B22BC053C954A4CA7CF451872E9C5B971F0DA5D92133648226D02E3ABB611
                  P2P_ADDRESS: 0.0.0.0:1337
                  API_ADDRESS: 0.0.0.0:8080
                  GENESIS_PUBLIC_KEY: ed01203420F48A9EEB12513B8EB7DAF71979CE80A1013F5F341C10DCDA4F6AA19F97A9
                  GENESIS_SIGNED_FILE: /tmp/genesis.signed.scale
                  SUMERAGI_TRUSTED_PEERS: '[{"address":"irohad2:1339","public_key":"ed0120222832FD8DF02882F07C13554DBA5BAE10C07A97E4AE7C2114DC05E95C3E6E32"},{"address":"irohad1:1338","public_key":"ed0120ACD30C7213EF11C4EC1006C6039E4089FC39C9BD211F688B866BCA59C8073883"},{"address":"irohad3:1340","public_key":"ed0120FB35DF84B28FAF8BB5A24D6910EFD7D7B22101EB99BFC74C4213CB1E7215F91B"}]'
                ports:
                - 1337:1337
                - 8080:8080
                volumes:
                - ./config:/config
                init: true
                command: |-
                  /bin/sh -c "
                  kagami genesis sign /config/genesis.json --public-key $$GENESIS_PUBLIC_KEY --private-key 8026405A6D5F06A90D29AD906E2F6EA8B41B4EF187849D0D397081A4A15FFCBE71E7C73420F48A9EEB12513B8EB7DAF71979CE80A1013F5F341C10DCDA4F6AA19F97A9 --out-file $$GENESIS_SIGNED_FILE &&
                  irohad --submit-genesis
                  "
                healthcheck:
                  test: test $(curl -s http://127.0.0.1:8080/status/blocks) -gt 0
                  interval: 2s
                  timeout: 1s
                  retries: 30
                  start_period: 4s
              irohad1:
                build: ./iroha-cloned
                platform: linux/amd64
                environment:
                  CHAIN_ID: 00000000-0000-0000-0000-000000000000
                  PUBLIC_KEY: ed0120ACD30C7213EF11C4EC1006C6039E4089FC39C9BD211F688B866BCA59C8073883
                  PRIVATE_KEY: 80264083CA4DC66124BB71EB7498FFCD0BFE981554F9E1426133A9999400A65B8D5636ACD30C7213EF11C4EC1006C6039E4089FC39C9BD211F688B866BCA59C8073883
                  P2P_ADDRESS: 0.0.0.0:1338
                  API_ADDRESS: 0.0.0.0:8081
                  GENESIS_PUBLIC_KEY: ed01203420F48A9EEB12513B8EB7DAF71979CE80A1013F5F341C10DCDA4F6AA19F97A9
                  SUMERAGI_TRUSTED_PEERS: '[{"address":"irohad2:1339","public_key":"ed0120222832FD8DF02882F07C13554DBA5BAE10C07A97E4AE7C2114DC05E95C3E6E32"},{"address":"irohad0:1337","public_key":"ed0120AB0B22BC053C954A4CA7CF451872E9C5B971F0DA5D92133648226D02E3ABB611"},{"address":"irohad3:1340","public_key":"ed0120FB35DF84B28FAF8BB5A24D6910EFD7D7B22101EB99BFC74C4213CB1E7215F91B"}]'
                ports:
                - 1338:1338
                - 8081:8081
                volumes:
                - ./config:/config
                init: true
                healthcheck:
                  test: test $(curl -s http://127.0.0.1:8081/status/blocks) -gt 0
                  interval: 2s
                  timeout: 1s
                  retries: 30
                  start_period: 4s
              irohad2:
                build: ./iroha-cloned
                platform: linux/amd64
                environment:
                  CHAIN_ID: 00000000-0000-0000-0000-000000000000
                  PUBLIC_KEY: ed0120222832FD8DF02882F07C13554DBA5BAE10C07A97E4AE7C2114DC05E95C3E6E32
                  PRIVATE_KEY: 802640DC39D66EF98C566D1641B5C971C17385EA6CBFD9029627C481AD777A2E31B7BD222832FD8DF02882F07C13554DBA5BAE10C07A97E4AE7C2114DC05E95C3E6E32
                  P2P_ADDRESS: 0.0.0.0:1339
                  API_ADDRESS: 0.0.0.0:8082
                  GENESIS_PUBLIC_KEY: ed01203420F48A9EEB12513B8EB7DAF71979CE80A1013F5F341C10DCDA4F6AA19F97A9
                  SUMERAGI_TRUSTED_PEERS: '[{"address":"irohad0:1337","public_key":"ed0120AB0B22BC053C954A4CA7CF451872E9C5B971F0DA5D92133648226D02E3ABB611"},{"address":"irohad1:1338","public_key":"ed0120ACD30C7213EF11C4EC1006C6039E4089FC39C9BD211F688B866BCA59C8073883"},{"address":"irohad3:1340","public_key":"ed0120FB35DF84B28FAF8BB5A24D6910EFD7D7B22101EB99BFC74C4213CB1E7215F91B"}]'
                ports:
                - 1339:1339
                - 8082:8082
                volumes:
                - ./config:/config
                init: true
                healthcheck:
                  test: test $(curl -s http://127.0.0.1:8082/status/blocks) -gt 0
                  interval: 2s
                  timeout: 1s
                  retries: 30
                  start_period: 4s
              irohad3:
                build: ./iroha-cloned
                platform: linux/amd64
                environment:
                  CHAIN_ID: 00000000-0000-0000-0000-000000000000
                  PUBLIC_KEY: ed0120FB35DF84B28FAF8BB5A24D6910EFD7D7B22101EB99BFC74C4213CB1E7215F91B
                  PRIVATE_KEY: 8026409F42DA01C65923DF0703284BF76AD475D6EE141A9C2F75C9C534DA11835D77D8FB35DF84B28FAF8BB5A24D6910EFD7D7B22101EB99BFC74C4213CB1E7215F91B
                  P2P_ADDRESS: 0.0.0.0:1340
                  API_ADDRESS: 0.0.0.0:8083
                  GENESIS_PUBLIC_KEY: ed01203420F48A9EEB12513B8EB7DAF71979CE80A1013F5F341C10DCDA4F6AA19F97A9
                  SUMERAGI_TRUSTED_PEERS: '[{"address":"irohad2:1339","public_key":"ed0120222832FD8DF02882F07C13554DBA5BAE10C07A97E4AE7C2114DC05E95C3E6E32"},{"address":"irohad0:1337","public_key":"ed0120AB0B22BC053C954A4CA7CF451872E9C5B971F0DA5D92133648226D02E3ABB611"},{"address":"irohad1:1338","public_key":"ed0120ACD30C7213EF11C4EC1006C6039E4089FC39C9BD211F688B866BCA59C8073883"}]'
                ports:
                - 1340:1340
                - 8083:8083
                volumes:
                - ./config:/config
                init: true
                healthcheck:
                  test: test $(curl -s http://127.0.0.1:8083/status/blocks) -gt 0
                  interval: 2s
                  timeout: 1s
                  retries: 30
                  start_period: 4s
        "#]];
        expected.assert_eq(&yaml);
    }
}
