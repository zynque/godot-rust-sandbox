shader_type canvas_item;

uniform sampler2D render_texture;

void fragment() {
    COLOR = texture(render_texture, UV);
}
