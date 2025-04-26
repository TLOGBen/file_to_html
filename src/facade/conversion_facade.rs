use std::io;
use std::path::Path;
use crate::config::ports::ConfigPort;
use crate::models::conversion::{ConversionInput, ConversionOutput};
use crate::models::file::{FileCollectInput, FileCollectOutput};
use crate::models::zip::{ZipCompressInput, ZipCompressOutput};
use crate::models::html::{HtmlGenerateInput};
use crate::service::config_service::ConfigService;
use zip::write::SimpleFileOptions;
use zip::AesMode;
use log::info;
use crate::facade::traits::i_conversion::ConversionFacadeTrait;
use crate::service::traits::i_service::{FileServiceTrait, HtmlServiceTrait, ZipServiceTrait};

pub struct ConversionFacade {
    config_service: ConfigService,
    file_service: Box<dyn FileServiceTrait>,
    zip_service: Box<dyn ZipServiceTrait>,
    html_service: Box<dyn HtmlServiceTrait>,
}

impl ConversionFacade {
    pub fn new(
        config_port: Box<dyn ConfigPort>,
        file_service: Box<dyn FileServiceTrait>,
        zip_service: Box<dyn ZipServiceTrait>,
        html_service: Box<dyn HtmlServiceTrait>,
    ) -> Self {
        let config_service = ConfigService::new(config_port);
        ConversionFacade {
            config_service,
            file_service,
            zip_service,
            html_service,
        }
    }
}

impl ConversionFacadeTrait for ConversionFacade {
    fn execute_conversion(&self, input: ConversionInput) -> io::Result<ConversionOutput> {
        let config = self.config_service.get_config()?;
        let input_path = Path::new(&input.input_path);
        let output_dir = &input.output_dir;

        let file_input = FileCollectInput {
            input_path: input.input_path.clone(),
            include_patterns: input.include.clone(),
            exclude_patterns: input.exclude.clone(),
            max_size: input.max_size,
            no_progress: input.no_progress,
        };

        let file_output = if input.is_compressed {
            self.file_service.collect_files(file_input)?
        } else {
            self.file_service.collect_files(file_input)?
        };

        let processed_files = file_output.files.len();
        if processed_files == 0 {
            log::warn!("無符合條件的檔案可處理");
            return Ok(ConversionOutput {
                output_path: input.output_dir.clone(),
                processed_files: 0,
            });
        }

        if input.is_compressed {
            info!("開始壓縮轉換，輸入路徑：{}，輸出目錄：{}", input.input_path.display(), input.output_dir);
            self.process_compressed(input.clone(), &file_output)?;
        } else {
            info!("開始個別轉換，輸入路徑：{}，輸出目錄：{}", input.input_path.display(), input.output_dir);
            self.process_individual(input.clone(), &file_output)?;
        }

        Ok(ConversionOutput {
            output_path: input.output_dir.clone(),
            processed_files,
        })
    }
}

impl ConversionFacade {
    fn process_compressed(&self, input: ConversionInput, file_output: &FileCollectOutput) -> io::Result<()> {
        std::fs::create_dir_all(&input.output_dir)?;
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::DEFLATE)
            .compression_level(Some(5));

        let password = crate::utils::utils::generate_password(&input.password_mode, None)?;
        let aes_mode = match input.encryption_method.as_str() {
            "aes128" => AesMode::Aes128,
            "aes192" => AesMode::Aes192,
            "aes256" => AesMode::Aes256,
            _ => AesMode::Aes256,
        };

        let zip_input = ZipCompressInput {
            files: file_output.files.clone(),
            input_path: input.input_path.clone(),
            options,
            password: password.clone(),
            aes_mode,
            no_progress: input.no_progress,
        };

        let zip_output = self.zip_service.compress_files(zip_input)?;
        self.finalize_compression(input, &zip_output, file_output.total_size, password.as_deref(), aes_mode)?;
        Ok(())
    }

    fn process_individual(&self, input: ConversionInput, file_output: &FileCollectOutput) -> io::Result<()> {
        std::fs::create_dir_all(&input.output_dir)?;
        let password = crate::utils::utils::generate_password(&input.password_mode, None)?;
        let aes_mode = match input.encryption_method.as_str() {
            "aes128" => AesMode::Aes128,
            "aes192" => AesMode::Aes192,
            "aes256" => AesMode::Aes256,
            _ => AesMode::Aes256,
        };

        for file_path in &file_output.files {
            let html_input = HtmlGenerateInput {
                zip_buffer: self.compress_single_file(file_path, &input, password.clone(), aes_mode)?,
                input_path: file_path.clone(),
                output_dir: input.output_dir.clone(),
                layer: input.layer.clone(),
                password: password.clone(),
                display_password: input.display_password,
                total_size: file_output.total_size,
            };
            self.html_service.generate_html(html_input)?;
        }
        Ok(())
    }

    fn compress_single_file(
        &self,
        file_path: &Path,
        input: &ConversionInput,
        password: Option<String>,
        aes_mode: AesMode,
    ) -> io::Result<Vec<u8>> {
        let (data, _file_size) = crate::service::file::read_file_content(file_path)?;
        let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
        let zip_input = ZipCompressInput {
            files: vec![file_path.to_path_buf()],
            input_path: file_path.to_path_buf(),
            options: SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::DEFLATE)
                .compression_level(Some(5)),
            password,
            aes_mode,
            no_progress: input.no_progress,
        };
        let zip_output = self.zip_service.compress_files(zip_input)?;
        Ok(zip_output.zip_buffer)
    }

    fn finalize_compression(
        &self,
        input: ConversionInput,
        zip_output: &ZipCompressOutput,
        total_size: usize,
        password: Option<&str>,
        aes_mode: AesMode,
    ) -> io::Result<()> {
        let html_input = HtmlGenerateInput {
            zip_buffer: zip_output.zip_buffer.clone(),
            input_path: input.input_path.clone(),
            output_dir: input.output_dir.clone(),
            layer: input.layer.clone(),
            password: password.map(String::from),
            display_password: input.display_password,
            total_size,
        };
        self.html_service.generate_html(html_input)?;
        Ok(())
    }
}