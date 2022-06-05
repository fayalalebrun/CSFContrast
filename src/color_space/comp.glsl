#version 430 core

#define LOCAL_SIZE 64

layout (local_size_x = LOCAL_SIZE, local_size_y = 1, local_size_z = 1) in;

layout (binding = 0, rgba32f) uniform image2D image;




uniform uint mode;

void main()
{
  ivec2 imgSize = imageSize(image);
  
  ivec2 pixel_coord = ivec2(gl_WorkGroupID.x*LOCAL_SIZE + gl_LocalInvocationID.x, gl_WorkGroupID.y);
  if (pixel_coord.x >= imgSize.x) {
    return;
  }
  
  // BT.709
  mat4 RGBtoYCbCr = mat4(0.2126, 0.7152, 0.0722, 0.0,
			 -0.1146, -0.3854, 0.5, 0.0,
			 0.5, -0.4542, -0.0458, 0.0,
			 0.0, 0.0, 0.0, 1.0);

  mat4 YCbCrtoRGB = mat4 (1.0, 0.0, 1.5748, 0.0,
			  1.0, -0.1873, -0.4681, 0.0,
			  1.0, 1.8556, 0.0, 0.0,
			  0.0, 0.0, 0.0, 1.0);
  
  vec4 color = imageLoad(image, pixel_coord);
  
  switch (mode)
    {
    case 0:
      {
	color = RGBtoYCbCr * color;
	break;
      }
    case 1:
      {
	color = YCbCrtoRGB * color;
	// FIXME
	color.b = color.r;
	color.g = color.r;
	break;
      }
    }

  imageStore(image, pixel_coord, color);
}
