// TODO have main actually get input, run command, upload output

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    // 1. get the record id from the command line for the project
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <record_id>", args[0]);
        std::process::exit(1);
    }
    let record_id = &args[1];
    // 2. transform the cut job record into the type `CutList`
    let cut_list = get_cut_list(record_id).await?;
    // 3. download the input files from the input into a temporary directory
    download_input_files(&cut_list).await?;
    // 4. build and run the command
    let temp_output_file = tempfile::NamedTempFile::new()?;
    run_command(&cut_list, temp_output_file.path()).await?;
    // 5. upload the output file to the output bucket
    upload_output_file(temp_output_file.path()).await?;
    // 6. update the job record with the output file url
    update_job_record(record_id).await?;
    Ok(())
}

async fn get_cut_list(record_id: &str) -> Result<CutList, Box<dyn std::error::Error>> {
    // TODO
    Ok(CutList {
        input_files: vec![],
        output_file: "".to_string(),
        command: "".to_string(),
    })
}

async fn download_input_files(cut_list: &CutList) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    Ok(())
}

async fn run_command(cut_list: &CutList, output_file: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    Ok(())
}

async fn upload_output_file(output_file: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    Ok(())
}

async fn update_job_record(record_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // TODO
    Ok(())
}

