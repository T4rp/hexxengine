#!/bin/bash

glslc shaders/main.vert -o assets/base.vert.spv
glslc shaders/main.frag -o assets/base.frag.spv

glslc shaders/shadow.vert -o assets/shadow.vert.spv
glslc shaders/shadow.frag -o assets/shadow.frag.spv

glslc shaders/skybox.vert -o assets/skybox.vert.spv
glslc shaders/skybox.frag -o assets/skybox.frag.spv

