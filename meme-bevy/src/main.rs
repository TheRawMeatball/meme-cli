use arboard::Clipboard;
use bevy::{
    ecs::prelude::*,
    input::Input,
    math::*,
    prelude::{App, Assets, Children, GlobalTransform, Handle, MouseButton, Transform},
    render2::{
        camera::OrthographicCameraBundle,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::Image,
    },
    sprite2::{PipelinedSpriteBundle, Sprite},
    window::Windows,
    PipelinedDefaultPlugins,
};
use memeinator::Config;
use ui4::plugin::{Ui4Plugin, Ui4Root};

mod ui;

#[derive(ui4::prelude::Lens, Default)]
pub(crate) struct TextRects {
    rects: ui4::prelude::TrackedVec<Entity>,
}

#[derive(ui4::prelude::Lens, Default)]
pub(crate) struct MemeName(String);
#[derive(PartialEq, Clone, Copy)]
enum MemeTextColor {
    Black,
    White,
}

#[derive(Default, Component)]
struct TextRect {
    min: IVec2,
    max: IVec2,
}

struct OnePxHandle(Handle<Image>);

fn main() {
    let memecfg = Config::load().unwrap();
    let clipboard = Clipboard::new().unwrap();

    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        // .add_plugin(bevy_inspector_egui::WorldInspectorPlugin::default())
        .add_plugin(Ui4Plugin)
        .init_resource::<TextRects>()
        .init_resource::<MemeName>()
        .insert_resource(MemeTextColor::Black)
        .add_plugin(Ui4Root(ui::root))
        .insert_resource(memecfg)
        .insert_resource(clipboard)
        .add_startup_system(
            |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                let display_handle = images.add(Image::new_fill(
                    Extent3d {
                        width: 600,
                        height: 600,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    &[0, 0, 0, 255],
                    TextureFormat::Rgba8UnormSrgb,
                ));
                commands
                    .spawn_bundle(PipelinedSpriteBundle {
                        texture: display_handle.clone(),
                        ..Default::default()
                    })
                    .insert(MemeSprite(1.));

                commands.spawn_bundle(OrthographicCameraBundle {
                    transform: Transform::from_xyz(0., 0., 10.),
                    ..OrthographicCameraBundle::new_2d()
                });

                commands.insert_resource(OnePxHandle(images.add(Image::new(
                    Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    vec![255; 4],
                    TextureFormat::Rgba8UnormSrgb,
                ))))
            },
        )
        .add_system(keep_on_edge_system.label("koe"))
        .add_system(handle_system.after("koe").label("hs"))
        .add_system(text_rect_system.after("hs"))
        .run();
}

#[derive(Component)]
struct MemeSprite(f32); // aspect ratio (w / h)

fn keep_on_edge_system(
    mut q: Query<(&mut Sprite, &mut Transform, &MemeSprite)>,
    windows: Res<Windows>,
) {
    if let Some((width, height)) = windows.get_primary().map(|x| (x.width(), x.height())) {
        for (mut sprite, mut pos, aspect_ratio) in q.iter_mut() {
            let available_x = width - 360.; // from UI
            let size = if height * aspect_ratio.0 < available_x {
                Vec2::new(height * aspect_ratio.0, height)
            } else {
                Vec2::new(available_x, available_x / aspect_ratio.0)
            };
            pos.translation.x = (width - size.x) / 2.;
            sprite.custom_size = Some(size * 0.9);
        }
    }
}

fn text_rect_system(
    mut rects_q: Query<
        (&mut Sprite, &mut Transform, &TextRect, &Children),
        (Without<MemeSprite>, Without<RectHandle>),
    >,
    mut handle_q: Query<(Entity, &mut Transform, &RectHandle), Without<MemeSprite>>,
    meme_sprite: Query<(&Sprite, &Handle<Image>, &Transform), With<MemeSprite>>,
    images: Res<Assets<Image>>,
    windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
    mut commands: Commands,
) {
    let mpos = windows.get_primary().and_then(|w| {
        w.cursor_position().map(|mpos| {
            let wsize = Vec2::new(w.width(), w.height());
            mpos - wsize / 2.
        })
    });
    let (meme_sprite, handle, meme_pos) = meme_sprite.single();
    let meme_sprite_size = meme_sprite.custom_size.unwrap();
    let image_size = images.get(handle).unwrap().texture_descriptor.size;
    let image_size = UVec2::new(image_size.width, image_size.height).as_vec2();
    let scale = meme_sprite_size.x / image_size.x; // aspect ratio preserved, so ratio same for both axes

    let size = meme_sprite_size;

    for (mut sprite, mut transform, rect, handles) in rects_q.iter_mut() {
        let min = Vec2::Y * size.y + Vec2::new(1., -1.) * rect.min.as_vec2() * scale;
        let max = Vec2::Y * size.y + Vec2::new(1., -1.) * rect.max.as_vec2() * scale;

        let rect_size = max - min;

        let pos = (max + min - size) / 2.;
        transform.translation = pos.extend(1.);

        sprite.custom_size = Some(rect_size);
        for &handle in handles.iter() {
            let (entity, mut handle_transform, handle) = handle_q.get_mut(handle).unwrap();
            let hpos =
                rect_size * Vec2::new(handle.x.get_mult() as f32, -handle.y.get_mult() as f32) / 2.;
            handle_transform.translation = hpos.extend(2.);
            if input.just_pressed(MouseButton::Left) {
                if let Some(mpos) = mpos {
                    let gpos = hpos + pos + meme_pos.translation.truncate();
                    if ((mpos - (gpos)).abs().cmplt(Vec2::ONE * 20.)).all() {
                        commands.entity(entity).insert(ActiveHandle {
                            offset: mpos - gpos,
                        });
                    }
                }
            }
        }
    }
}

fn handle_system(
    mut handle_q: Query<(Entity, &GlobalTransform, &RectHandle, &ActiveHandle)>,
    mut text_item_q: Query<&mut TextRect>,
    windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
    mut commands: Commands,

    images: Res<Assets<Image>>,
    meme_sprite: Query<(&Sprite, &Handle<Image>), With<MemeSprite>>,
) {
    let mpos = windows.get_primary().and_then(|w| {
        w.cursor_position().map(|mpos| {
            let wsize = Vec2::new(w.width(), w.height());
            mpos - wsize / 2.
        })
    });
    let active = handle_q.get_single_mut();
    if let (Some(mpos), Ok((entity, transform, handle, active))) = (mpos, active) {
        if input.pressed(MouseButton::Left) {
            let (meme_sprite, meme_img_handle) = meme_sprite.single();
            let meme_sprite_size = meme_sprite.custom_size.unwrap();
            let image_size = images.get(meme_img_handle).unwrap().texture_descriptor.size;
            let image_size = UVec2::new(image_size.width, image_size.height).as_vec2();
            let scale = image_size.x / meme_sprite_size.x; // aspect ratio preserved, so ratio same for both axes

            let delta = mpos - active.offset - transform.translation.truncate();
            let mut text_item = text_item_q.get_mut(handle.rect).unwrap();
            let delta = delta * scale;
            match handle.x {
                HandleSide::Indifferent => {}
                HandleSide::Positive => text_item.max.x += delta.x as i32,
                HandleSide::Negative => text_item.min.x += delta.x as i32,
                HandleSide::Both => {
                    text_item.max.x += delta.x as i32;
                    text_item.min.x += delta.x as i32;
                }
            }
            match handle.y {
                HandleSide::Indifferent => {}
                HandleSide::Positive => text_item.min.y -= delta.y as i32,
                HandleSide::Negative => text_item.max.y -= delta.y as i32,
                HandleSide::Both => {
                    text_item.min.y -= delta.y as i32;
                    text_item.max.y -= delta.y as i32;
                }
            }
        } else if input.just_released(MouseButton::Left) {
            commands.entity(entity).remove::<ActiveHandle>();
        }
    }
}

#[derive(Copy, Clone)]
enum HandleSide {
    Indifferent,
    Positive,
    Negative,
    Both,
}

impl HandleSide {
    fn get_mult(self) -> i8 {
        match self {
            Self::Indifferent => 0,
            Self::Positive => 1,
            Self::Negative => -1,
            Self::Both => 0,
        }
    }
}

#[derive(Component)]
struct RectHandle {
    rect: Entity,
    x: HandleSide,
    y: HandleSide,
}

#[derive(Component)]
struct ActiveHandle {
    offset: Vec2,
}
