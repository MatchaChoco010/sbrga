#version 460 core

layout (location = 0) in vec2 Position;
layout (location = 1) in vec4 Color;

out vec4 vColor;

uniform mat4 ViewProjection;

void main() {
  vColor = Color;
  gl_Position = ViewProjection * vec4(Position, 0.0, 1.0);
}
