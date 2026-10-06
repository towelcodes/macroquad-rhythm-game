#version 100
precision highp float;

varying vec2 uv;
uniform vec2 iResolution;
uniform float speed;
uniform vec4 _Time;

void main() {
    float iTime = _Time.x;
    vec2 n = uv;
    n.x *= iResolution.x / iResolution.y;

    vec2 w = n + vec2(iTime * speed * -1.0, iTime * speed);
    float check = mod(floor(w.x * 12.0) + floor(w.y * 12.0), 2.0);

    vec3 blue  = vec3(0.62, 0.76, 0.93);
    vec3 cream = vec3(0.96, 0.94, 0.85);
    vec3 col = mix(cream, blue, check);

    gl_FragColor = vec4(col, 1.0);
}
