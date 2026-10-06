#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color;

varying lowp vec2 uv;
varying lowp vec4 vert_color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
    vert_color = color;
}
