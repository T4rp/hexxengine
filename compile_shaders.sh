#!/bin/bash

glslc -fshader-stage=vert -DVERTEX shaders/uber.glsl  -o shaders/base.vert.spv
glslc -fshader-stage=frag -DFRAGMENT shaders/uber.glsl -o shaders/base.frag.spv
