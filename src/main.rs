extern crate delta_e;
extern crate lab;

use delta_e::DE2000;
use fs::read_to_string;
use lab::Lab;
use std::{fs::{self, File}, vec};
use std::path::Path;
use std::io::BufWriter;
use clap::Clap;
use std::io::Write;

#[derive(Clap)]
#[clap(version = "1.0")]
struct Opts{
    file: Option<String>,
    #[clap(default_value = "output.png")]
    output_location: String,
    #[clap(short, long, about = "use dithering")]
    dither: bool,
    #[clap(short, long, about =" whether to use staircase method or not, staircase method usually produces much better results, but is significantly more complicated")]
    staircase: bool,
    #[clap(short, long, about = "whether to only use the blocks listed in \"mask.txt\"")]
    use_mask: bool,
}


fn main() {
    let options: Opts = Opts::parse();

    let mut map_colors: Vec<[u8; 3]> = vec![];
    let mut color_names: Vec<String> = vec![];

    let mut csv_reader = csv::Reader::from_reader(File::open("blockdata.csv").expect("unable to ready blockdata.csv"));

    let mut mask: Vec<&str> = vec![];

    let mask_file = std::fs::read_to_string("mask.txt").expect("unable to read mask.txt");

    if options.use_mask{
        mask = mask_file.lines().collect::<Vec<&str>>();
    }

    // load color data
    for (index, result) in csv_reader.records().enumerate() {

        let record = result.expect("trouble loading csv");

        let name  = record.get(3).expect(&format!("missing block name at line {}", index + 1));
        if options.use_mask && !mask.contains(&name){
            continue;
        }

        let r_opt = record.get(0).expect(&format!("missing red value at line {}", index + 1)).parse::<u8>();
        let g_opt = record.get(1).expect(&format!("missing green value at line {}", index + 1)).parse::<u8>();
        let b_opt = record.get(2).expect(&format!("missing blue value at line {}", index + 1)).parse::<u8>();

        let r = r_opt.expect(&format!("red value at line {} is invalid", index + 1));
        let g = g_opt.expect(&format!("green value at line {} is invalid", index + 1));
        let b = b_opt.expect(&format!("blue value at line {} is invalid", index + 1));

        // level color
        map_colors.push([(r as f32 * 0.86) as u8, (g as f32 * 0.86) as u8, (b as f32 * 0.86) as u8]);
        color_names.push(name.to_owned() + " LEVEL");

        if options.staircase{
            // up color
            map_colors.push([r, g, b]);
            color_names.push(name.to_owned() + " UP");

            // down color
            map_colors.push([(r as f32 * 0.71) as u8, (g as f32 * 0.71) as u8, (b as f32 * 0.71) as u8]);
            color_names.push(name.to_owned() + " DOWN");
        }

    }
    
    let file = File::open(options.file.expect("filename not provided"));

    let decoder = png::Decoder::new(file.expect("unable to open file"));
    let data = decoder.read_info();
    let (info, mut reader) = data.expect("unable to decode PNG");

    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf).expect("unable to read frame");

    let bytes_per_pixel = reader.info().bytes_per_pixel();
    println!("bytes per pixel: {}", bytes_per_pixel);

    let mut color_indicies :Vec<usize> = vec![];

    for y in 0..reader.info().height{
        for x in 0..reader.info().width{
            
            let index: usize = (y * reader.info().width + x) as usize;
            // skip transparent pixels
            if bytes_per_pixel == 4 && buf[bytes_per_pixel * index + 3] == 0{
                continue;
            }
            let old_pixel = [buf[bytes_per_pixel * index], buf[bytes_per_pixel * index + 1], buf[bytes_per_pixel * index + 2]];
            let p = get_closest_color(old_pixel, &map_colors);
            let new_pixel = p.0;
            let color_index = p.1;
            color_indicies.push(color_index);

            buf[bytes_per_pixel * index] = new_pixel[0];
            buf[bytes_per_pixel * index + 1] = new_pixel[1];
            buf[bytes_per_pixel * index + 2] = new_pixel[2];
            let quant_error = [old_pixel[0] as f32 - new_pixel[0] as f32, old_pixel[1] as f32 - new_pixel[1] as f32, old_pixel[2] as f32 - new_pixel[2] as f32];

            if options.dither{
                // pixel[x + 1][y    ] := pixel[x + 1][y    ] + quant_error × 7 / 16
                if x != reader.info().width - 1 { // if not on right edge
                    let part = 7.0 / 16.0;
                    let other_index = index + 1; // move right one pixel
                    buf[bytes_per_pixel * other_index] = u8::saturating_add(((quant_error[0] as f32) * part) as u8, buf[bytes_per_pixel * other_index]);
                    buf[bytes_per_pixel * other_index + 1] = u8::saturating_add(((quant_error[1] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 1]);
                    buf[bytes_per_pixel * other_index + 2] = u8::saturating_add(((quant_error[2] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 2]);
                }
                // pixel[x - 1][y + 1] := pixel[x - 1][y + 1] + quant_error × 3 / 16
                if x != 0 && y != reader.info().height - 1 { // if not on left edge or bottom edge
                    let part = 3.0 / 16.0;
                    let other_index = index - 1 + reader.info().width as usize; // move left one pixel and down one pixel
                    buf[bytes_per_pixel * other_index] = u8::saturating_add(((quant_error[0] as f32) * part) as u8, buf[bytes_per_pixel * other_index]);
                    buf[bytes_per_pixel * other_index + 1] = u8::saturating_add(((quant_error[1] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 1]);
                    buf[bytes_per_pixel * other_index + 2] = u8::saturating_add(((quant_error[2] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 2]);
                }
                // pixel[x    ][y + 1] := pixel[x    ][y + 1] + quant_error × 5 / 16
                if y != reader.info().height - 1 { // if not on bottom edge
                    let part = 5.0 / 16.0;
                    let other_index = index + reader.info().width as usize; // move down one pixel
                    buf[bytes_per_pixel * other_index] = u8::saturating_add(((quant_error[0] as f32) * part) as u8, buf[bytes_per_pixel * other_index]);
                    buf[bytes_per_pixel * other_index + 1] = u8::saturating_add(((quant_error[1] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 1]);
                    buf[bytes_per_pixel * other_index + 2] = u8::saturating_add(((quant_error[2] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 2]);
                }
                // pixel[x + 1][y + 1] := pixel[x + 1][y + 1] + quant_error × 1 / 16
                if x != reader.info().width -1 && y != reader.info().height - 1 { // if not on right edge or bottom edge
                    let part = 1.0 / 16.0;
                    let other_index = index + 1 + reader.info().width as usize; // move right one pixel and down one pixel
                    buf[bytes_per_pixel * other_index] = u8::saturating_add(((quant_error[0] as f32) * part) as u8, buf[bytes_per_pixel * other_index]);
                    buf[bytes_per_pixel * other_index + 1] = u8::saturating_add(((quant_error[1] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 1]);
                    buf[bytes_per_pixel * other_index + 2] = u8::saturating_add(((quant_error[2] as f32) * part) as u8, buf[bytes_per_pixel * other_index + 2]);
                }
            }
        }
    }

    let instructions_file_option = File::create("instructions.txt");

    let mut instructions_file = instructions_file_option.expect("unable to create instructions.txt");
    let mut materials_list: Vec<usize> = vec![];
    if options.staircase{
        materials_list.resize(map_colors.len() / 3, 0);
    }else{
        materials_list.resize(map_colors.len(), 0);
    }

    for x in 0..reader.info().width {
        for y in (0..reader.info().height){
            let mut color_name = &color_names[color_indicies[(y * reader.info().width + x) as usize]];
            if options.staircase{
                materials_list[color_indicies[(y * reader.info().width + x) as usize] / 3] += 1;
            }else{
                //println!("x: {}, y: {}", x, y);
                materials_list[color_indicies[(y * reader.info().width + x) as usize]] += 1;
            }
            instructions_file.write_all(format!("{}\n", color_name).as_bytes());
        }
        if (x != reader.info().width - 1){
            instructions_file.write_all(b"return to bottom\n");
        }else{
            instructions_file.write_all(b"finished!");
        }
    }

    let materials_file_option = File::create("materials.txt");
    let mut materials_file = materials_file_option.expect("unable to create materials.txt");

    let mut first = true;

    for (i, material_count) in materials_list.iter().enumerate(){
        if *material_count == 0 as usize{
            continue;
        }
        let mut color_string = "ERROR";
        if options.staircase{
            color_string = color_names[i * 3].trim_end_matches(" LEVEL");
        }else{
            color_string = color_names[i].trim_end_matches(" LEVEL");
        }

        if first {
            materials_file.write_all(format!("{}x {}", material_count, color_string).as_bytes());
            first = false;
        }else{
            materials_file.write_all(format!("\n{}x {}", material_count, color_string).as_bytes());
        }
    }



    let path = Path::new(&options.output_location);
    let file = File::create(path).expect("unable to create file at output location");
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, reader.info().width, reader.info().height);
    if bytes_per_pixel == 3 {
        encoder.set_color(png::ColorType::RGB);
    }else if bytes_per_pixel == 4 {
        encoder.set_color(png::ColorType::RGBA);
    }else{
        panic!("invalid number of bytes per pixel!")
    }
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&buf).unwrap();

}

fn get_closest_color(color: [u8; 3], colors: &Vec<[u8; 3]>) -> ([u8; 3], usize){
    let mut closest_color = [0 as u8, 0 as u8, 0 as u8];
    let mut closest_index = 0;
    let mut de = 10000000.0;
    for (index, c) in colors.iter().enumerate(){
        let delta_e = DE2000::from_rgb(&color, &c);
        if delta_e < de{
            closest_color = *c;
            de = delta_e;
            closest_index = index;
        }
    }
    (closest_color, closest_index)
}