use base64::Engine as _;
use color_eyre::eyre;
use forgejo_api::{
    Forgejo,
    structs::{
        CreateBranchRepoOption, CreateFileOptions, CreateRepoOption, CreateUserOption, Identity,
    },
};
use std::{
    io::{self, Write},
    str::FromStr,
};
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
// tokio::io not needed here; tests use their own async runtime
use url::Url;

/// TODO: Broken!
pub struct ForgejoContainer {
    _container: ContainerAsync<GenericImage>,
    token: String,
    port: u16,
}

impl ForgejoContainer {
    pub async fn with_tag(tag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let forgejo_image = GenericImage::new("codeberg.org/forgejo/forgejo", tag)
            .with_exposed_port(3000.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Starting new Web server"))
            .with_env_var("FORGEJO__server__ROOT_URL", "http://0.0.0.0:3000/")
            .with_env_var("FORGEJO__security__INSTALL_LOCK", "true")
            .with_env_var(
                "FORGEJO__service__ALLOW_ONLY_EXTERNAL_REGISTRATION",
                "false",
            );

        let container = forgejo_image.start().await?;
        let port = container.get_host_port_ipv4(3000).await?;
        let mut exec_result = container
        .exec(ExecCommand::new(vec![
            "su",
            "-c",
            "forgejo admin user create --admin --username containeradmin --password testtest --email test@example.com --access-token",
            "git"
        ]))
        .await?;

        let stdout_buf = exec_result.stdout_to_vec().await?;
        let stdout_str = String::from_utf8_lossy(&stdout_buf);
        io::stdout().write_all(&stdout_buf)?;

        let stderr_buf = exec_result.stderr_to_vec().await?;
        io::stderr().write_all(&stderr_buf)?;

        // Extract the access token from stdout
        let token_line = stdout_str
            .lines()
            .find(|line| line.contains("Access token was successfully created"))
            .expect("Failed to find access token line");

        let token = token_line
            .split("...")
            .nth(1)
            .expect("Failed to extract token")
            .trim()
            .to_string();

        println!("Intercepted Forgejo token");
        Ok(Self {
            _container: container,
            port,
            token,
        })
    }
    pub async fn api(&self) -> Forgejo {
        let addr = format!("http://{}:{}", "localhost", self.port);
        let addr = Url::from_str(&addr).unwrap();
        Forgejo::new(forgejo_api::Auth::Token(&self.token), addr).unwrap()
    }

    /// Return the mapped HTTP port for the Forgejo instance.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Return the intercepted API token.
    pub fn token(&self) -> &str {
        &self.token
    }

    pub async fn add_asset(
        &self,
        owner: &str,
        project: &str,
        channel: &str,
        path: &str,
        asset: &[u8],
    ) {
        let port = self.port;
        let token = self.token.clone();
        let owner = owner.to_owned();
        let project = project.to_owned();
        let channel = channel.to_owned();
        let path = path.to_owned();
        let asset_vec = asset.to_vec();

        let addr = format!("http://{}:{}", "localhost", port);
        let addr = Url::from_str(&addr).unwrap();
        let api = Forgejo::new(forgejo_api::Auth::Token(&token), addr).unwrap();

        if api.user_get(&owner).await.is_err() {
            api.admin_create_user(CreateUserOption {
                created_at: None,
                email: format!("{}@example.com", &owner),
                full_name: None,
                login_name: Some(owner.clone()),
                must_change_password: None,
                password: Some("giasfclfebrehber".to_string()),
                restricted: None,
                send_notify: None,
                source_id: None,
                username: owner.clone(),
                visibility: None,
            })
            .await
            .unwrap();
        }

        if api.repo_get(&owner, &project).await.is_err() {
            let _ = api
                .create_current_user_repo(CreateRepoOption {
                    name: project.clone(),
                    description: Some("created by test-utils add_asset".to_string()),
                    private: Some(false),
                    auto_init: Some(true),
                    default_branch: Some("staticcontainerbranch".to_string()),
                    gitignores: None,
                    issue_labels: None,
                    license: Some("MIT".to_string()),
                    object_format_name: None,
                    readme: None,
                    template: None,
                    trust_model: None,
                })
                .await
                .unwrap();
        }

        let branch = api
            .repo_get_branch(&owner, &project, "staticcontainerbranch")
            .await
            .unwrap();

        let branch = match api.repo_get_branch(&owner, &project, &channel).await {
            Ok(v) => v,
            Err(_) => api
                .repo_create_branch(
                    &owner,
                    &project,
                    CreateBranchRepoOption {
                        new_branch_name: channel.clone(),
                        old_branch_name: Some("staticcontainerbranch".to_string()),
                        old_ref_name: Some("staticcontainerbranch".to_string()),
                    },
                )
                .await
                .unwrap(),
        };

        //    let content_b64 = base64::engine::general_purpose::STANDARD.encode(&asset_vec);

        let author = Identity {
            email: Some(format!("{}@example.com", owner)),
            name: Some(owner.clone()),
        };

        let opts = CreateFileOptions {
            branch: Some(channel),
            content: base64::encode(&asset_vec),
            message: Some(format!("Add asset {}", path)),
            author: Some(author),
            committer: None,
            dates: None,
            new_branch: None,
            signoff: None,
        };

        // Try to create the file; if it already exists, attempt an update.
        api.repo_create_file(&owner, &project, &path, opts)
            .await
            .unwrap();
    }

    /*pub async fn write_project_source<PS: Upstream + 'static>(
        &self,
        ps: &PS,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let api = Arc::new(self.api().await);
        let ps = Arc::new(ps);

        // Step 1: Get all owners and convert to owned Vec
        let owners = ps.list_owners().await?.collect::<Vec<_>>();

        // Create a stream over owners
        stream::iter(owners)
            .for_each(|owner_name| {
                let api = api.clone();
                async move {

                    if api.user_get(&owner_name).await.is_err() {
                        api.admin_create_user(CreateUserOption {
                            created_at: None,
                            email: format!("{}@example.com", &owner_name),
                            full_name: None,
                            login_name: Some(owner_name.clone()),
                            must_change_password: None,
                            password: Some("giasfclfebrehber".to_string()),
                            restricted: None,
                            send_notify: None,
                            source_id: None,
                            username: owner_name.clone(),
                            visibility: None,
                        })
                        .await
                        .unwrap();
                    }

                    // Collect projects to own them
                    let projects = match ps.list_projects(&owner_name).await {
                        Ok(p) => p.collect::<Vec<_>>(),
                        Err(_) => return, // handle error as needed
                    };

                    // Stream over projects
                    stream::iter(projects)
                        .for_each(|project_name| {
                            let api = api.clone();
                            let owner_name = owner_name.clone();
                            async move {
                                // Check if repo exists
                                if api.repo_get(&owner_name, &project_name).await.is_err() {
                                    // Ensure the repo is initialized with a default branch so
                                    // subsequent branch creation doesn't fail due to missing
                                    // source reference in the Git backend.
                                    api.create_current_user_repo(CreateRepoOption {
                                        name: project_name.clone(),
                                        description: Some(
                                            "Imported from ProjectSource".to_string(),
                                        ),
                                        private: Some(false),
                                        // create an initial commit and definite default branch
                                        auto_init: Some(true),
                                        default_branch: Some("main".to_string()),
                                        gitignores: None,
                                        issue_labels: None,
                                        license: None,
                                        object_format_name: None,
                                        readme: None,
                                        template: None,
                                        trust_model: None,
                                    })
                                    .await
                                    .unwrap();
                                }

                                // Collect channels to own them
                                let channels = match ps.list_channels(&owner_name, &project_name).await {
                                    Ok(c) => c.collect::<Vec<_>>(),
                                    Err(_) => return,
                                };

                                // Stream over channels
                                stream::iter(channels)
                                    .for_each(|branch_name| {
                                        let api = api.clone();
                                        let owner_name = owner_name.clone();
                                        let project_name = project_name.clone();
                                        async move {

                                            if api
                                                .repo_get_branch(
                                                    &owner_name,
                                                    &project_name,
                                                    &branch_name,
                                                )
                                                .await
                                                .is_err()
                                            {
                                                api.repo_create_branch(
                                                    &owner_name,
                                                    &project_name,
                                                    CreateBranchRepoOption {
                                                        new_branch_name: branch_name.clone(),
                                                        old_branch_name: None,
                                                        old_ref_name: None,
                                                    },
                                                )
                                                .await
                                                .unwrap(); // API ERR: Target couldn't be found - old_ref_name is required (cannot create a branch from nothing in API)
                                                // Potential solution; Create and keep a special empty branch at initialization that is not intended to be normally used (test_static_abcdef) and have everything else branch from that
                                            }

                                            if let Ok(asset_paths) = channel.asset_keys().await {
                                                for path in asset_paths {
                                                    if let Ok(asset) =
                                                        channel.get_asset(Path::new(&path)).await
                                                    {
                                                        let path =
                                                            Path::new(&path).normalized_relative();
                                                        let content = asset.as_slice();
                                                        let path = path.to_str().unwrap();
                                                        let _ = api
                                                            .repo_create_file(
                                                                &owner_name,
                                                                &project_name,
                                                                path,
                                                                CreateFileOptions {
                                                                    branch: Some(
                                                                        branch_name.clone(),
                                                                    ),
                                                                    content: base64::engine::general_purpose::STANDARD
                                                                        .encode(content),
                                                                    message: Some(format!(
                                                                        "Add asset {:?}",
                                                                        path
                                                                    )),
                                                                    author: Some(Identity {
                                                                        email: Some(format!(
                                                                            "{}@example.com",
                                                                            owner_name
                                                                        )),
                                                                        name: Some(
                                                                            owner_name.clone(),
                                                                        ),
                                                                    }),
                                                                    committer: None,
                                                                    dates: None,
                                                                    new_branch: None,
                                                                    signoff: None,
                                                                },
                                                            )
                                                            .await
                                                            .unwrap();
                                                    }
                                                }
                                            }
                                        }
                                    })
                                    .await;
                            }
                        })
                        .await;
                }
            })
            .await;

        Ok(())
    }*/
}
