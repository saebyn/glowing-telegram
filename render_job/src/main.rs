use job_utils::*;

mod job_utils;

/**
 * This program will take a project that describes a video editing job and
 * and will run the job using ffmpeg and upload the result to S3 and update
 * the project record with the output file path.
 */
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Read configuration from environment variables with figment
    let app_context = initialize_app_context().await?;

    // 1. get the record ids from the command line for the projects
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <record_id1> <record_id2> ...", args[0]);
        std::process::exit(1);
    }
    let record_ids = &args[1..];

    // 2. transform the cut job records into the type `CutList`
    let mut cut_lists = Vec::new();
    for record_id in record_ids {
        let cut_list = get_cut_list(&app_context, record_id).await?;
        cut_lists.push(cut_list);
    }

    // 3. download the input files from the input into a temporary directory
    let temp_input_dir = tempfile::tempdir()?;
    download_input_files(&app_context, &cut_lists, temp_input_dir.path())
        .await?;

    // 4. build and run the commands
    let mut temp_output_files = Vec::new();
    for cut_list in cut_lists {
        let temp_output_file = tempfile::NamedTempFile::new()?;
        run_command(&cut_list, temp_input_dir.path(), temp_output_file.path())
            .await?;
        temp_output_files.push(temp_output_file);
    }

    // 5. upload the output files to the output bucket
    let mut output_locations = Vec::new();
    for temp_output_file in temp_output_files {
        let output_location =
            upload_output_file(&app_context, temp_output_file.path()).await?;
        output_locations.push(output_location);
    }

    // 6. update the job records with the output file locations
    for (record_id, output_location) in record_ids.iter().zip(output_locations)
    {
        update_job_record(&app_context, record_id, output_location).await?;
    }

    Ok(())
}
