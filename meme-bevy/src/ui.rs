use arboard::Clipboard;
use bevy::{
    prelude::*,
    render2::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::Image,
    },
    sprite2::{PipelinedSpriteBundle, Sprite},
};
use memeinator::{Config, MemeConfig, MemeField};
use ui4::{
    lens::{ComponentLens, LensObserver},
    prelude::*,
};

use crate::{HandleSide, MemeSprite, OnePxHandle, RectHandle, TextRect, TextRects};

use super::{MemeName, MemeTextColor};

pub fn root(ctx: Ctx) -> Ctx {
    ctx.with(UiColor(Color::BLACK))
        .with(Width(Units::Pixels(360.)))
        .with(Height(Units::Percentage(100.)))
        .children(
            res::<TextRects>()
                .lens(TextRects::rects)
                .each(|entity, index: IndexObserver| {
                    move |ctx: &mut McCtx| {
                        ctx.dyn_group(index.map_child(|index| {
                            move |ctx: &mut McCtx| {
                                if index != 0 {
                                    ctx.c(separator);
                                }
                            }
                        }));
                        ctx.c(move |ctx: Ctx| text_size_repr_elem(entity, index, ctx));
                    }
                }),
        )
        .child(top_buttons)
}

fn text_size_repr_elem(
    entity: impl WorldLens<Out = Entity>,
    index: IndexObserver,
    ctx: Ctx,
) -> Ctx {
    let item_text = |f: &'static (dyn Fn(&TextRect) -> i32 + Send + Sync)| {
        text(
            entity
                .map(|&entity: &Entity| component::<TextRect>(entity))
                .flatten()
                .map(
                    move |text_item: FlattenReturn<LensObserver<ComponentLens<TextRect>>>| {
                        f(&**text_item).to_string()
                    },
                ),
        )
    };
    ctx.with(LayoutType::Row)
        .with(Height(Units::Pixels(60.)))
        .child(|ctx| {
            ctx.with(LayoutType::Column)
                .with(Width(Units::Pixels(300.)))
                .child(|ctx| {
                    ctx.with(LayoutType::Row)
                        .child(text("Top-left:").with(Width(Units::Pixels(100.))))
                        .child(item_text(&(|text| text.min.x)).with(Width(Units::Pixels(100.))))
                        .child(item_text(&(|text| text.min.y)).with(Width(Units::Pixels(100.))))
                })
                .child(|ctx| {
                    ctx.with(LayoutType::Row)
                        .child(text("Bottom-right:").with(Width(Units::Pixels(100.))))
                        .child(item_text(&(|text| text.max.x)).with(Width(Units::Pixels(100.))))
                        .child(item_text(&(|text| text.max.y)).with(Width(Units::Pixels(100.))))
                })
        })
        .child(
            button("remove text")
                .with(Width(Units::Pixels(60.)))
                .with(Height(Units::Pixels(60.)))
                .with(index.dedup().map(|&index: &usize| {
                    OnClick::new(move |w| {
                        let entity = w
                            .get_resource_mut::<TextRects>()
                            .unwrap()
                            .rects
                            .remove(index);

                        w.entity_mut(entity).despawn_recursive();
                    })
                })),
        )
}

fn top_buttons(mut ctx: Ctx) -> Ctx {
    let mut new_text_state = ctx.state::<(
        Commands,
        Res<OnePxHandle>,
        ResMut<TextRects>,
        Query<Entity, With<MemeSprite>>,
    )>();

    let mut save_meme_state = ctx.state::<(
        Res<TextRects>,
        ResMut<MemeName>,
        Res<MemeTextColor>,
        ResMut<Assets<Image>>,
        ResMut<Config>,
        Query<&Handle<Image>, With<MemeSprite>>,
        Query<&TextRect>,
    )>();

    let mut clipboard_get_state = ctx.state::<(
        ResMut<Clipboard>,
        ResMut<Assets<Image>>,
        Query<(&mut Handle<Image>, &mut MemeSprite)>,
    )>();

    ctx.child(button("New Text").with(OnClick::new(move |world| {
        let (mut commands, image, mut rects, q) = new_text_state.get_mut(world);
        let root = q.single();
        let e = commands
            .spawn_bundle(PipelinedSpriteBundle {
                texture: image.0.clone(),
                sprite: Sprite {
                    color: Color::rgba(0.8, 0.8, 0.8, 0.4),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(TextRect {
                min: IVec2::new(100, 100),
                max: IVec2::new(200, 200),
            })
            .with_children(|parent| {
                let e = parent.parent_entity();
                let list = {
                    use HandleSide::*;
                    [
                        (Negative, Negative),
                        (Positive, Positive),
                        (Negative, Positive),
                        (Positive, Negative),
                        (Positive, Indifferent),
                        (Negative, Indifferent),
                        (Indifferent, Positive),
                        (Indifferent, Negative),
                        (Both, Both),
                    ]
                };
                for (x, y) in list {
                    parent
                        .spawn_bundle(PipelinedSpriteBundle {
                            texture: image.0.clone(),
                            sprite: Sprite {
                                color: Color::BLUE,
                                custom_size: Some(Vec2::new(20., 20.)),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .insert(RectHandle { rect: e, x, y });
                }
            })
            .id();
        commands.entity(root).push_children(&[e]);
        rects.rects.push(e);

        new_text_state.apply(world);
    })))
    .child(|ctx| {
        ctx.with(LayoutType::Row)
            .with(Height(Units::Pixels(30.)))
            .child(text("Template name").with(Width(Units::Pixels(100.))))
            .child(textbox(res::<MemeName>().lens(MemeName::F0)).with(Width(Units::Pixels(260.))))
    })
    .child(|ctx| {
        fn rb(color: MemeTextColor, t: &'static str) -> impl FnOnce(Ctx) -> Ctx {
            move |ctx: Ctx| {
                ctx.with(LayoutType::Row)
                    .with(Width(Units::Pixels(130.)))
                    .child(text(t).with(Width(Units::Pixels(100.))))
                    .child(
                        radio_button(color, res::<MemeTextColor>()).with(Width(Units::Pixels(30.))),
                    )
            }
        }
        ctx.with(LayoutType::Row)
            .with(Height(Units::Pixels(30.)))
            .child(text("Text Color").with(Width(Units::Pixels(100.))))
            .child(rb(MemeTextColor::Black, "Black"))
            .child(rb(MemeTextColor::White, "White"))
    })
    .child(button("Save Template").with(OnClick::new(move |w| {
        let (rects, mut meme_name, color, images, config, meme_sprite, rects_q) =
            save_meme_state.get_mut(w);

        if meme_name.0.is_empty() {
            print!("Can't save template without name");
            return;
        }

        let image = images.get(meme_sprite.single()).unwrap();
        let text = rects
            .rects
            .iter()
            .map(|&rect| rects_q.get(rect).unwrap())
            .map(|rect| MemeField {
                min: (
                    rect.min.x.min(rect.max.x) as u32,
                    rect.min.y.min(rect.max.y) as u32,
                ),
                max: (
                    rect.min.x.max(rect.max.x) as u32,
                    rect.min.y.max(rect.max.y) as u32,
                ),
            })
            .collect();

        let width = image.texture_descriptor.size.width;
        let height = image.texture_descriptor.size.height;
        let error = config.write_template(
            &image.data,
            width,
            height,
            MemeConfig {
                color: Some(match *color {
                    MemeTextColor::Black => [0., 0., 0., 1.],
                    MemeTextColor::White => [1., 1., 1., 1.],
                }),
                text,
            },
            &meme_name.0,
        );

        if let Err(e) = error {
            println!("{:#}", e);
        } else {
            meme_name.0.clear();
            w.resource_scope(|world, mut rects: Mut<TextRects>| {
                for &rect in &*rects.rects {
                    world.entity_mut(rect).despawn_recursive();
                }
                rects.rects.clear();
            });
        }
    })))
    .child(
        button("Get template from clipboard").with(OnClick::new(move |world| {
            let (mut clipboard, mut images, mut q) = clipboard_get_state.get_mut(world);
            if let Ok(img) = clipboard.get_image() {
                let (mut handle, mut koe) = q.single_mut();
                koe.0 = img.width as f32 / img.height as f32;

                *handle = images.add(Image::new(
                    Extent3d {
                        width: img.width as u32,
                        height: img.height as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    img.bytes.into_owned(),
                    TextureFormat::Rgba8UnormSrgb,
                ));
            }
        })),
    )
}

fn separator(ctx: Ctx) -> Ctx {
    ctx.with(Height(Units::Pixels(2.)))
        .with(Width(Units::Stretch(1.)))
        .with(Left(Units::Pixels(10.)))
        .with(Right(Units::Pixels(10.)))
        .with(Top(Units::Pixels(2.)))
        .with(Bottom(Units::Pixels(2.)))
        .with(UiColor(Color::DARK_GRAY))
}
