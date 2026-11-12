// Shelf thumbnails: each generator, smart mask and brush alpha shown as a small picture made by the same code that
// applies it (the procedural generators, the smart-mask stacks and the brush tip), on a stand-in surface so the ones
// that follow edges, crevices or up-facing parts show what they do. Made once each and kept (egui temp data).

use eframe::egui;
use crate::brush::{tip_alpha, AlphaType};
use crate::channel::{RgbaBuffer, ScalarBuffer};
use crate::layer::{Layer, ProceduralGenerator};
use crate::shelf::{ShelfAsset, ShelfCategory};
use glam::{Vec3, Vec4};
use std::collections::HashMap;

/// Thumbnail size in pixels (drawn at about half that on screen, so it stays sharp on high-DPI displays).
pub const THUMB: u32 = 80;

/// The stand-in surface the previews are made on: a tile with bevelled raised edges and a cross-shaped groove, lit
/// from above (curvature, ambient occlusion, position and world normal maps).
pub struct SampleMaps {
    pub curvature: ScalarBuffer,
    pub ao: ScalarBuffer,
    pub position: RgbaBuffer,
    pub normal: RgbaBuffer,
}

pub fn sample_maps(n: u32) -> SampleMaps {
    let mut curvature = ScalarBuffer::new(n, n, 0.5);
    let mut ao = ScalarBuffer::new(n, n, 1.0);
    let mut position = RgbaBuffer::new(n, n, Vec4::ONE);
    let mut normal = RgbaBuffer::new(n, n, Vec4::new(0.5, 0.5, 1.0, 1.0));
    for y in 0..n {
        for x in 0..n {
            let u = (x as f32 + 0.5) / n as f32;
            let v = (y as f32 + 0.5) / n as f32;
            let i = (y * n + x) as usize;
            let (b, g) = shape(u, v);
            curvature.data[i] = (0.5 + 0.45 * b * b - 0.42 * g).clamp(0.0, 1.0);
            ao.data[i] = (1.0 - 0.8 * g - 0.12 * b).clamp(0.0, 1.0);
            // Up is the top of the picture.
            position.data[i] = Vec4::new(u, 1.0 - v, 0.5, 1.0);
            // (A dome from top to bottom: its upper part faces the sky, so dust and other up-facing masks show.)
            let nrm = Vec3::new((u - 0.5) * 0.7, (0.5 - v) * 3.2, 1.0).normalize();
            normal.data[i] = Vec4::new(nrm.x * 0.5 + 0.5, nrm.y * 0.5 + 0.5, nrm.z * 0.5 + 0.5, 1.0);
        }
    }
    SampleMaps { curvature, ao, position, normal }
}

/// (bevel 0-1, groove 0-1) at (u, v) on the stand-in tile.
fn shape(u: f32, v: f32) -> (f32, f32) {
    let e = u.min(1.0 - u).min(v).min(1.0 - v);
    let bevel = 1.0 - (e / 0.13).clamp(0.0, 1.0);
    let groove = 1.0 - (((u - 0.5).abs().min((v - 0.5).abs())) / 0.045).clamp(0.0, 1.0);
    (bevel, groove * (1.0 - bevel))
}

/// The generator a "Noises" shelf item applies (as in PaintCraftApp::apply_shelf_asset).
pub fn generator_for(id: &str) -> ProceduralGenerator {
    match id {
        "gen_perlin" => ProceduralGenerator::PerlinNoise,
        "gen_worley" => ProceduralGenerator::WorleyGrunge,
        "gen_billow" => ProceduralGenerator::BillowNoise,
        "gen_ridged" => ProceduralGenerator::RidgedMultifractal,
        "gen_domain_warp" => ProceduralGenerator::DomainWarpGrunge,
        "gen_splatter" => ProceduralGenerator::SplatterDirt,
        "gen_wood" => ProceduralGenerator::WoodGrain,
        "gen_fabric" => ProceduralGenerator::FabricWeave,
        "gen_scratches" => ProceduralGenerator::Scratches,
        "gen_curvature" => ProceduralGenerator::CurvatureWear,
        "gen_cavity" => ProceduralGenerator::AmbientOcclusion,
        "gen_dust" => ProceduralGenerator::DirectionalDust,
        "gen_leaks" => ProceduralGenerator::DrippingLeaks,
        "gen_position" => ProceduralGenerator::PositionGradient,
        "gen_ground_dirt" => ProceduralGenerator::GroundSplash,
        "gen_checker" => ProceduralGenerator::Checkerboard,
        "gen_bricks" => ProceduralGenerator::Bricks,
        "gen_perforated" => ProceduralGenerator::PerforatedMesh,
        "gen_wire_fence" => ProceduralGenerator::WireFence,
        "gen_torn_fabric" => ProceduralGenerator::TornFabricEdge,
        "gen_dots" => ProceduralGenerator::CircularDots,
        "gen_bnw_spots_1" => ProceduralGenerator::BnWSpots1,
        "gen_bnw_spots_2" => ProceduralGenerator::BnWSpots2,
        "gen_bnw_spots_3" => ProceduralGenerator::BnWSpots3,
        "gen_grunge_map" => ProceduralGenerator::GrungeMap,
        "gen_directional_noise" => ProceduralGenerator::DirectionalNoise,
        "gen_clouds" => ProceduralGenerator::Clouds,
        "gen_marble_veins" => ProceduralGenerator::MarbleVeins,
        "gen_moisture" => ProceduralGenerator::MoistureNoise,
        _ => ProceduralGenerator::PerlinNoise,
    }
}

fn alpha_for(id: &str) -> Option<AlphaType> {
    Some(match id {
        "alpha_soft_round" => AlphaType::SoftRound,
        "alpha_hard_round" => AlphaType::HardRound,
        "alpha_noise_splatter" => AlphaType::NoiseSplatter,
        "alpha_chisel" => AlphaType::Chisel,
        "alpha_stipple" => AlphaType::Stipple,
        "alpha_scratches" => AlphaType::Scratches,
        _ => return None,
    })
}

/// The mask a smart mask (or a mesh-driven generator) makes on the stand-in tile, 0-1 per pixel.
fn mask_values(id: &str, n: u32, m: &SampleMaps) -> Vec<f32> {
    let mut layer = Layer::new_paint_layer(0, String::new(), n, n);
    let (c, a, p, w) = (Some(&m.curvature), Some(&m.ao), Some(&m.position), Some(&m.normal));
    match id {
        "gen_curvature" => { layer.has_mask = true; layer.add_mask_generator_curvature_wear(c, a, p, w); }
        "gen_cavity" => { layer.has_mask = true; layer.add_mask_generator_cavity_dirt(c, a, p, w); }
        "gen_dust" => { layer.has_mask = true; layer.add_mask_generator_directional_dust(c, a, p, w); }
        "gen_ground_dirt" => { layer.has_mask = true; layer.add_mask_generator_ground_dirt(c, a, p, w); }
        "gen_mask_editor" => { layer.has_mask = true; layer.add_mask_generator_mask_editor(c, a, p, w); }
        _ => crate::smart_materials::SmartMaterialLibrary::apply_smart_mask_advanced(id, &mut layer, c, a, p, w),
    }
    layer.evaluate_mask_full(c, a, p, w, None, &[]);
    let inv = layer.mask_inverted;
    layer.mask_buf.data.iter().map(|&v| if inv { 1.0 - v } else { v }.clamp(0.0, 1.0)).collect()
}

/// Stretched to use the whole range, and lifted when what shows is only a few thin bright marks, so a faint or sparse
/// pattern still reads at thumbnail size. (Only the picture: the generator itself is unchanged.)
fn levels(mut v: Vec<f32>) -> Vec<f32> {
    let (lo, hi) = v.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    if hi - lo > 0.02 { for x in v.iter_mut() { *x = (*x - lo) / (hi - lo); } }
    let mean = v.iter().sum::<f32>() / v.len().max(1) as f32;
    if mean < 0.18 { for x in v.iter_mut() { *x = x.powf(0.55); } }
    v
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn to_image(n: u32, px: impl Fn(u32, u32) -> [f32; 3]) -> egui::ColorImage {
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let c = px(x, y);
            for k in c {
                out.push((k.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
            }
            out.push(255);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([n as usize, n as usize], &out)
}

const BG: [f32; 3] = [0.075, 0.08, 0.095];
const INK: [f32; 3] = [0.9, 0.91, 0.93];

/// The thumbnail for a shelf item, or None for kinds that draw their own (materials).
pub fn render(asset: &ShelfAsset, n: u32) -> Option<egui::ColorImage> {
    match asset.category {
        ShelfCategory::Generators => {
            let maps = sample_maps(n);
            let mesh_driven = matches!(asset.id, "gen_curvature" | "gen_cavity" | "gen_dust" | "gen_ground_dirt" | "gen_mask_editor");
            let vals = if mesh_driven {
                mask_values(asset.id, n, &maps)
            } else {
                let gen = generator_for(asset.id);
                let scale = match gen {
                    ProceduralGenerator::Checkerboard | ProceduralGenerator::Bricks => 4.0,
                    ProceduralGenerator::PerforatedMesh | ProceduralGenerator::WireFence | ProceduralGenerator::CircularDots => 5.0,
                    _ => 3.0,
                };
                Layer::generate_mask_procedural(gen, scale, 1.0, false, 0.5, n, n, Some(&maps.curvature), Some(&maps.ao), Some(&maps.position), Some(&maps.normal)).data
            };
            let vals = levels(vals);
            Some(to_image(n, |x, y| mix(BG, INK, vals[(y * n + x) as usize])))
        }
        ShelfCategory::SmartMasks => {
            let maps = sample_maps(n);
            let vals = if asset.id == "mask_invert" {
                // A pattern, its right half inverted.
                let base = Layer::generate_mask_procedural(ProceduralGenerator::BnWSpots1, 3.0, 1.0, false, 0.5, n, n, None, None, None, None).data;
                (0..(n * n)).map(|i| if i % n >= n / 2 { 1.0 - base[i as usize] } else { base[i as usize] }).collect()
            } else {
                levels(mask_values(asset.id, n, &maps))
            };
            let tint = mix([asset.preview_color[0] as f32 / 255.0, asset.preview_color[1] as f32 / 255.0, asset.preview_color[2] as f32 / 255.0], [1.0; 3], 0.3);
            Some(to_image(n, |x, y| {
                let u = (x as f32 + 0.5) / n as f32;
                let v = (y as f32 + 0.5) / n as f32;
                // The tile, lightly shaded so its edges and groove read, under the mask in the mask's colour.
                let (b, g) = shape(u, v);
                let shade = 0.17 + 0.05 * (1.0 - v) + 0.07 * b - 0.09 * g;
                let base = [shade * 0.92, shade * 0.95, shade * 1.05];
                mix(base, tint, vals[(y * n + x) as usize])
            }))
        }
        ShelfCategory::Alphas => {
            let t = alpha_for(asset.id)?;
            let r = n as f32 * 0.4;
            let half = n as f32 / 2.0;
            Some(to_image(n, |x, y| {
                // 2 x 2 samples a pixel, so hard edges are smooth.
                let mut a = 0.0;
                for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                    let px = ((x as f32 + sx - half) * 2.0).round() as i32;
                    let py = ((y as f32 + sy - half) * 2.0).round() as i32;
                    a += tip_alpha(t, px, py, r * 2.0, 0.35);
                }
                mix(BG, INK, a / 4.0)
            }))
        }
        _ => None,
    }
}

/// Made thumbnails, kept in egui's temporary data for the session.
#[derive(Clone, Default)]
struct Cache(HashMap<&'static str, egui::TextureHandle>);

/// The thumbnail texture for a shelf item (made the first time it's shown).
pub fn texture(ctx: &egui::Context, asset: &ShelfAsset) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("paintcraft_shelf_thumbs");
    if let Some(t) = ctx.data_mut(|d| d.get_temp_mut_or_default::<Cache>(id).0.get(asset.id).cloned()) {
        return Some(t);
    }
    let img = render(asset, THUMB)?;
    let tex = ctx.load_texture(format!("shelf_{}", asset.id), img, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| { d.get_temp_mut_or_default::<Cache>(id).0.insert(asset.id, tex.clone()); });
    Some(tex)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every generator, smart mask and alpha on the shelf gets a picture that isn't one flat colour.
    #[test]
    fn every_shelf_item_has_a_real_thumbnail() {
        for a in crate::shelf::get_shelf_assets() {
            if !matches!(a.category, ShelfCategory::Generators | ShelfCategory::SmartMasks | ShelfCategory::Alphas) {
                continue;
            }
            let img = render(a, 32).unwrap_or_else(|| panic!("no thumbnail for {}", a.id));
            assert_eq!(img.size, [32, 32]);
            let lum: Vec<f32> = img.pixels.iter().map(|p| p.r() as f32 + p.g() as f32 + p.b() as f32).collect();
            let (lo, hi) = lum.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
            assert!(hi - lo > 30.0, "{} thumbnail is flat ({}..{})", a.id, lo, hi);
        }
    }

    /// Writes every thumbnail into one picture, to look at them (`PAINTCRAFT_THUMBS=out.png cargo test -- --ignored`).
    #[test]
    #[ignore]
    fn contact_sheet() {
        let Ok(out) = std::env::var("PAINTCRAFT_THUMBS") else { return };
        let items: Vec<_> = crate::shelf::get_shelf_assets().iter().filter_map(|a| render(a, THUMB).map(|i| (a, i))).collect();
        let (cols, pad) = (8u32, 6u32);
        let cell = THUMB + pad;
        let rows = (items.len() as u32 + cols - 1) / cols;
        let mut img = image::RgbaImage::from_pixel(cols * cell + pad, rows * cell + pad, image::Rgba([30, 32, 36, 255]));
        for (n, (_a, t)) in items.iter().enumerate() {
            let k = n as u32;
            let cx = (k % cols) * cell + pad;
            let cy = (k / cols) * cell + pad;
            for y in 0..THUMB { for x in 0..THUMB {
                let p = t.pixels[(y * THUMB + x) as usize];
                img.put_pixel(cx + x, cy + y, image::Rgba([p.r(), p.g(), p.b(), 255]));
            } }
        }
        img.save(out).unwrap();
    }

    #[test]
    fn moisture_has_its_own_generator() {
        assert_eq!(generator_for("gen_moisture"), ProceduralGenerator::MoistureNoise);
    }
}
