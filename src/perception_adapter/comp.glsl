#version 450

#define LOCAL_SIZE 64

layout (local_size_x = LOCAL_SIZE, local_size_y = 1, local_size_z = 1) in;

layout (binding = 0, rgba32f) uniform image2D realPart;
layout (binding = 1, rgba32f) uniform image2D imagPart;


#define LUT_ARRAY_LEN 4096
layout(std430, binding=2) readonly buffer CsfLut {
  float lut_lower_limit;
  float lut_upper_limit;
  float lut_array[LUT_ARRAY_LEN];
};

uniform float pixels_per_visual_degree;

uniform float target_pixels_per_visual_degree;

float sampleLut(float x) {
  float adjusted = (x - lut_lower_limit)/ (lut_upper_limit - lut_lower_limit);
  adjusted = clamp(adjusted, 0.0, 1.0);

  return lut_array[uint((adjusted * float((LUT_ARRAY_LEN - 1))))];
}

// Computes the coordinates around N/2 if the cartesian quadrants are diagonally swapped
vec2 fftShift(ivec2 pixel_coord, ivec2 fftSize) {
  vec2 fpixel_coord = vec2(pixel_coord);
  vec2 adjusted = fpixel_coord - vec2(fftSize/2);
  vec2 coord =  adjusted + -1 * sign(adjusted + vec2(0.000000000000001))*vec2(fftSize/2) ;
  return coord;
}

// Gets dimensionless frequency
float freq(vec2 fft_coord, ivec2 fftSize) {
  
  // Intersect line from origin to coord with rectangle
  vec2 vertIntersect = vec2(fft_coord.x * (fftSize.y/2.0)/fft_coord.y,fftSize.y/2.0);
  vec2 horzIntersect = vec2(fftSize.x/2.0, fft_coord.y * (fftSize.x/2.0)/fft_coord.x);

  // Get the total length of this slice over the rectangle
  float N = 2.0 * min(length(vertIntersect), length(horzIntersect));

  
  // Frequency is index divided by points
  return length(fft_coord)/N;
}

void main() {
  ivec2 fftSize = imageSize(realPart);
  ivec2 pixel_coord = ivec2(gl_WorkGroupID.x * LOCAL_SIZE + gl_LocalInvocationID.x, gl_WorkGroupID.y);

  if (pixel_coord.x >= fftSize.x) {
    return;
  }

  vec2 fft_coord = fftShift(pixel_coord, fftSize);
  
  if (fft_coord == vec2(0.0)) { // Can't get a frequency at the origin
    return;
  }

  
  
  vec4 real = imageLoad(realPart, pixel_coord);
  vec4 imag = imageLoad(imagPart, pixel_coord);

  float magnitude = sqrt(real.x*real.x + imag.x*imag.x);
  float phase = atan(imag.x,real.x);

  // start of magnitude adjustment
  
  float freq = freq(fft_coord, fftSize);
  float cpd = freq * pixels_per_visual_degree;
  float target_cpd = freq * target_pixels_per_visual_degree;
  float cur_value = sampleLut(cpd);
  float target_value = sampleLut(target_cpd);

  float adjustment = target_value / cur_value;
  magnitude = adjustment * magnitude;

  // End of magnitude adjustment

  float realX = magnitude * cos(phase);
  float imagX = magnitude * sin(phase);

  real.x = realX;
  imag.x = imagX;
  
  imageStore(realPart, pixel_coord, real);
  imageStore(imagPart, pixel_coord, imag);

}
