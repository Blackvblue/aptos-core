// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_validators, k8s_wait_cli_strategy, k8s_wait_genesis_strategy, nodes_healthcheck, Result,
};
use ::aptos_logger::*;
use anyhow::{bail, format_err};
use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::Job,
};
use kube::{
    api::{Api, DeleteParams, ListParams, Meta},
    client::Client as K8sClient,
    Config,
};
use rand::Rng;
use rayon::prelude::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    convert::TryFrom,
    env,
    fs::File,
    io::Write,
    process::{Command, Stdio},
    str,
};
use tempfile::TempDir;

const HELM_BIN: &str = "helm";
const KUBECTL_BIN: &str = "kubectl";
const MAX_NUM_VALIDATORS: usize = 30;
const HEALTH_CHECK_URL: &str = "http://127.0.0.1:8001";

const GENESIS_MODULES_DIR: &str = "/aptos-framework/move/modules";

async fn wait_genesis_job(kube_client: &K8sClient, era: &str) -> Result<()> {
    aptos_retrier::retry_async(k8s_wait_genesis_strategy(), || {
        let jobs: Api<Job> = Api::namespaced(kube_client.clone(), "default");
        Box::pin(async move {
            let job_name = format!("genesis-aptos-genesis-e{}", era);
            debug!("Running get job: {}", &job_name);
            let genesis_job = jobs.get_status(&job_name).await.unwrap();
            debug!("Status: {:?}", genesis_job.status);
            let status = genesis_job.status.unwrap();
            match status.succeeded {
                Some(1) => {
                    println!("Genesis job completed");
                    Ok(())
                }
                _ => bail!("Genesis job not completed"),
            }
        })
    })
    .await
}

pub fn set_validator_image_tag(
    validator_name: String,
    image_tag: String,
    helm_repo: String,
) -> Result<()> {
    let validator_upgrade_options = vec![
        "--reuse-values".to_string(),
        "--history-max".to_string(),
        "2".to_string(),
        "--set".to_string(),
        format!("imageTag={}", image_tag),
    ];
    upgrade_validator(validator_name, helm_repo, &validator_upgrade_options)
}

/// Deletes a collection of resources in k8s
async fn delete_k8s_collection<T: Clone + DeserializeOwned + Meta>(
    api: Api<T>,
    name: &'static str,
) -> Result<()> {
    match api
        .delete_collection(&DeleteParams::default(), &ListParams::default())
        .await?
    {
        either::Left(list) => {
            let names: Vec<_> = list.iter().map(Meta::name).collect();
            println!("Deleting collection of {}: {:?}", name, names);
        }
        either::Right(status) => {
            println!("Deleted collection of {}: status={:?}", name, status);
        }
    }

    Ok(())
}

pub(crate) async fn delete_k8s_cluster() -> Result<()> {
    let client: K8sClient = create_k8s_client().await;
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), "default");
    let stateful_sets: Api<StatefulSet> = Api::namespaced(client, "default");

    // delete all deployments and statefulsets
    // cross this with all the compute resources created by aptos-node helm chart
    delete_k8s_collection(deployments, "deployments").await?;
    delete_k8s_collection(stateful_sets, "stateful_sets").await?;

    Ok(())
}

fn helm_release_patch(release_name: &str, version: usize) -> Result<()> {
    // trick helm into letting us upgrade later
    // https://phoenixnap.com/kb/helm-has-no-deployed-releases#ftoc-heading-5
    let helm_patch_args = [
        "patch",
        "secret",
        &format!("sh.helm.release.v1.{}.v{}", release_name, version),
        "--type=merge",
        "-p",
        "{\"metadata\":{\"labels\":{\"status\":\"deployed\"}}}",
    ];
    println!("{:?}", helm_patch_args);
    let helm_patch_output = Command::new(KUBECTL_BIN)
        .stdout(Stdio::inherit())
        .args(&helm_patch_args)
        .output()
        .expect("failed to kubectl patch secret valNN");
    assert!(
        helm_patch_output.status.success(),
        "{}",
        String::from_utf8(helm_patch_output.stderr).unwrap()
    );

    Ok(())
}

fn upgrade_helm_release(
    release_name: String,
    helm_chart: String,
    options: &[String],
) -> Result<()> {
    let upgrade_base_args = ["upgrade".to_string(), release_name, helm_chart];
    let upgrade_args = [&upgrade_base_args, options].concat();
    println!("{:?}", upgrade_args);
    let upgrade_output = Command::new(HELM_BIN)
        .stdout(Stdio::inherit())
        .args(&upgrade_args)
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "failed to helm upgrade release {} with chart {}",
                release_name, helm_chart
            )
        });
    if !upgrade_output.status.success() {
        bail!(format!(
            "Upgrade not completed: {}",
            String::from_utf8(upgrade_output.stderr).unwrap()
        ));
    }

    Ok(())
}

fn upgrade_validator(validator_name: String, helm_repo: String, options: &[String]) -> Result<()> {
    upgrade_helm_release(
        validator_name,
        format!("{}/aptos-validator", helm_repo),
        options,
    )
}

// runs helm upgrade on the installed aptos-genesis release named "genesis"
// if a new "era" is specified, a new genesis will be created, and old resources will be destroyed
fn upgrade_genesis_helm(helm_repo: &str, options: &[String]) -> Result<()> {
    upgrade_helm_release(
        "genesis".to_string(),
        format!("{}/aptos-genesis", helm_repo),
        options,
    )
}

fn get_helm_status(helm_release_name: &str) -> Result<Value> {
    let status_args = ["status", helm_release_name, "-o", "json"];
    println!("{:?}", status_args);
    let raw_helm_values = Command::new(HELM_BIN)
        .args(&status_args)
        .output()
        .unwrap_or_else(|_| panic!("failed to helm status {}", helm_release_name));

    let helm_values = String::from_utf8(raw_helm_values.stdout).unwrap();
    serde_json::from_str(&helm_values)
        .map_err(|e| format_err!("failed to deserialize helm values: {}", e))
}

fn get_helm_values(helm_release_name: &str) -> Result<Value> {
    let mut v: Value = get_helm_status(helm_release_name)
        .map_err(|e| format_err!("failed to helm get values aptos: {}", e))?;
    Ok(v["config"].take())
}

pub async fn uninstall_testnet_resources() -> Result<()> {
    // delete kubernetes resources
    delete_k8s_cluster().await?;
    println!("aptos-node resources removed");

    Ok(())
}

pub async fn reinstall_testnet_resources(
    helm_repo: String,
    base_num_validators: usize,
    base_validator_image_tag: String,
    base_genesis_image_tag: String,
    require_validator_healthcheck: bool,
    genesis_modules_path: Option<String>,
) -> Result<String> {
    assert!(base_num_validators <= MAX_NUM_VALIDATORS);

    let new_era = get_new_era().unwrap();
    let kube_client = create_k8s_client().await;

    // just helm upgrade

    let mut genesis_upgrade_options = vec![
        "--reuse-values".to_string(),
        "--history-max".to_string(),
        "2".to_string(),
        "--set".to_string(),
        format!("chain.era={}", &new_era),
        "--set".to_string(),
        format!("numValidators={}", base_num_validators),
        "--set".to_string(),
        format!("imageTag={}", &base_genesis_image_tag),
    ];

    // run genesis from the directory in aptos/init image
    if let Some(genesis_modules_path) = genesis_modules_path {
        genesis_upgrade_options.extend([
            "--set".to_string(),
            format!("genesis.moveModulesDir={}", genesis_modules_path).to_string(),
        ]);
    }

    // upgrade testnet
    upgrade_genesis_helm(&helm_repo, genesis_upgrade_options.as_slice())?;

    // wait for genesis to run again, and get the updated validators
    wait_genesis_job(&kube_client, &new_era).await.unwrap();
    let vals = get_validators(kube_client.clone(), &base_validator_image_tag)
        .await
        .unwrap();
    let all_nodes = vals.values().collect();
    // healthcheck on each of the validators wait until they all healthy
    let unhealthy_nodes = if require_validator_healthcheck {
        nodes_healthcheck(all_nodes).await.unwrap()
    } else {
        vec![]
    };
    if !unhealthy_nodes.is_empty() {
        bail!("Unhealthy validators after cleanup: {:?}", unhealthy_nodes);
    }

    Ok(new_era)
}

fn get_new_era() -> Result<String> {
    let v: Value = get_helm_values("aptos")?;
    println!("{}", v["chain"]["era"]);
    let chain_era: &str = &era_to_string(&v["chain"]["era"]).unwrap();

    // get the new era
    let mut rng = rand::thread_rng();
    let new_era: &str = &format!("fg{}", rng.gen::<u32>());
    println!("chain.era: {} --> {}", chain_era, new_era);
    Ok(new_era.to_string())
}

// sometimes helm will try to interpret era as a number in scientific notation
fn era_to_string(era_value: &Value) -> Result<String> {
    match era_value {
        Value::Number(num) => Ok(format!("{}", num)),
        Value::String(s) => Ok(s.to_string()),
        _ => bail!("Era is not a number {}", era_value),
    }
}

pub async fn create_k8s_client() -> K8sClient {
    let _ = Command::new(KUBECTL_BIN).arg("proxy").spawn();
    let _ = aptos_retrier::retry_async(k8s_wait_cli_strategy(), || {
        Box::pin(async move {
            debug!("Running local kube pod healthcheck on {}", HEALTH_CHECK_URL);
            reqwest::get(HEALTH_CHECK_URL).await?.text().await?;
            println!("Local kube pod healthcheck passed");
            Ok::<(), reqwest::Error>(())
        })
    })
    .await;
    let config = Config::new(
        reqwest::Url::parse(HEALTH_CHECK_URL).expect("Failed to parse kubernetes endpoint url"),
    );
    K8sClient::try_from(config).unwrap()
}

pub fn scale_sts_replica(sts_name: &str, replica_num: u64) -> Result<()> {
    let scale_sts_args = [
        "scale",
        "sts",
        sts_name,
        &format!("--replicas={}", replica_num),
    ];
    println!("{:?}", scale_sts_args);
    let scale_output = Command::new(KUBECTL_BIN)
        .stdout(Stdio::inherit())
        .args(&scale_sts_args)
        .output()
        .expect("failed to scale sts replicas");
    assert!(
        scale_output.status.success(),
        "{}",
        String::from_utf8(scale_output.stderr).unwrap()
    );

    Ok(())
}
