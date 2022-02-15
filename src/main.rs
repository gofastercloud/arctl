use aws_config::meta::region::RegionProviderChain;
use aws_sdk_apprunner::{Client, Error};

use clap::Parser;

use regex::Regex;

use chrono::prelude::*;

use std::process;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// List App Runner services
    #[clap(short, long)]
    list: bool,

    /// List supported AWS Regions for App Runner
    #[clap(short = 'L', long = "list-regions")]
    list_regions: bool,

    /// Describe App Runner service
    #[clap(short, long)]
    desc: bool,

    /// Delete App Runner service
    #[clap(long)]
    delete: bool,

    /// AppRunner service name
    #[clap(short, long)]
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = Client::new(&config);
    let args = Args::parse();

    let re = Regex::new("(?P<t>[a-zA-Z0-9-]+)..(?P<r>[a-z0-9-]+)..").unwrap();

    let get_region = format!("{:?}", config.region().unwrap());

    let configured_region = re.replace_all(&get_region, "$r");

    let ar_supported_regions = [
        "us-east-1",
        "us-east-2",
        "eu-west-1",
        "us-west-2",
        "ap-northeast-1",
    ];

    let mut supported: bool = false;

    for region in &ar_supported_regions {
        if &configured_region == region {
            supported = true;
            break;
        } else {
            continue;
        }
    }

    if args.list_regions {
        println!("Supported Regions for AWS AppRunner are:");
        for region in &ar_supported_regions {
            println!("{}", region)
        }
        println!("---");
        println!(
            "Your current profile is configured to use {}",
            configured_region
        );
        process::exit(0);
    }

    if !supported {
        println!(
            "{} is not currently supported by AWS AppRunner",
            configured_region
        );
        process::exit(1);
    }

    let req = client.list_services();
    let resp = req.send().await?;

    let services: &[aws_sdk_apprunner::model::ServiceSummary] =
        resp.service_summary_list().unwrap_or_default();

    if supported && args.list {
        if services.len() > 0 {
            println!(
                "AWS App Runner services currently running in {}",
                &configured_region
            );
            println!("---");
            for service in services {
                println!(
                    "{} - https://{}",
                    service.service_name().unwrap(),
                    service.service_url().unwrap()
                );
            }
            process::exit(0);
        } else {
            println!("No AWS App Runner services found");
            process::exit(2);
        }
    }

    if supported && args.desc {
        if args.name == None {
            println!("You must provide a Service Name");
            process::exit(3)
        }
        for service in services {
            if args.name.as_ref().unwrap() == service.service_name().unwrap() {
                let d_req = client
                    .describe_service()
                    .service_arn(service.service_arn().unwrap());
                let d_resp = d_req.send().await?;

                let service_details = d_resp.service().unwrap();

                println!(
                    "Service Name: {}",
                    service_details.service_name.as_ref().unwrap()
                );

                println!(
                    "Service ARN: {}",
                    service_details.service_arn.as_ref().unwrap()
                );

                let service_port = service_details
                    .source_configuration
                    .as_ref()
                    .unwrap()
                    .image_repository
                    .as_ref()
                    .unwrap()
                    .image_configuration
                    .as_ref()
                    .unwrap()
                    .port
                    .as_ref()
                    .unwrap();

                println!(
                    "Service URL: https://{}:{}",
                    service_details.service_url.as_ref().unwrap(),
                    service_port
                );

                let service_resources = service_details.instance_configuration.as_ref().unwrap();

                println!(
                    "System Resources: {} CPUs / {}MB RAM",
                    service_resources.cpu.as_ref().unwrap().parse().unwrap_or(0) / 1024,
                    service_resources.memory.as_ref().unwrap()
                );

                let timestamp = service_details.created_at.as_ref().unwrap().secs();
                let datetime =
                    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(timestamp, 0), Utc);
                let newdate = datetime.format("%Y-%m-%d %H:%M:%S");
                println!("Service created at: {} UTC", newdate);

                process::exit(0);
            }
        }
        println!(
            "Service {} not found in Region {}",
            args.name.unwrap(),
            configured_region
        )
    }

    Ok(())
}
