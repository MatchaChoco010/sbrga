#version 460 core

layout (location = 0) in vec3 Position;

uniform mat4 ViewProjection;

void main() {
  gl_Position = ViewProjection * vec4(Position, 1.0);
}
