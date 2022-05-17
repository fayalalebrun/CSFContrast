/// Adapted from https://github.com/bane9/OpenGLFFT/blob/main/OpenGLFFT/FFT2D.comp

#version 430 core

#define WORKGROUP_SIZE_X 256
#define SHARED_BUFFER_SIZE 4096

#define PI 3.14159265358979323846264338327950288

#define PIXEL_BUFFER_SIZE 32

layout (local_size_x = WORKGROUP_SIZE_X, local_size_y = 1, local_size_z = 1) in;

layout (binding = 0, rgba32f) uniform image2D inputImage;
layout (binding = 1, rgba32f) uniform image2D realPart;
layout (binding = 2, rgba32f) uniform image2D imagPart;

layout(std430, binding = 3) readonly buffer img_info {
	int input_width;
	int input_height;
	int output_width;
	int output_height;
	int logtwo_width;
	int logtwo_height;
	int clz_width;
	int clz_height;
	int no_of_channels;
};

uniform uint stage;


shared float real_cache[SHARED_BUFFER_SIZE];
shared float imag_cache[SHARED_BUFFER_SIZE];

vec4 pixel_buffer_real[PIXEL_BUFFER_SIZE];
vec4 pixel_buffer_imag[PIXEL_BUFFER_SIZE];

void sync()
{
    barrier();
    memoryBarrierShared();
}

vec2 cplx_mul(vec2 lhs, vec2 rhs) 
{
    return vec2(lhs.x * rhs.x - lhs.y * rhs.y, lhs.y * rhs.x + lhs.x * rhs.y);
}

uint rev_bits(uint num) 
{ 
    uint count = 31; 
    uint reverse_num = num; 
      
    num >>= 1;  
    while(num != 0) 
    { 
       reverse_num <<= 1;        
       reverse_num |= num & 1; 
       num >>= 1; 
       count--; 
    } 
    reverse_num <<= count; 
    return reverse_num; 
} 

uint index_map(uint threadId, uint currentIteration, uint N)
{
    return ((threadId & (N - (1u << currentIteration))) << 1) | (threadId & ((1u << currentIteration) - 1));
}

uint twiddle_map(uint threadId, uint currentIteration, uint logTwo, uint N)
{
    return (threadId & (N / (1u << (logTwo - currentIteration)) - 1)) * (1u << (logTwo - currentIteration)) >> 1;
}

vec2 twiddle(float q, bool is_inverse, float N)
{
	float theta = float(int(!is_inverse) * 2 - 1) * 2.0 * PI * q / N;

	float r = cos(theta);
	float i = sqrt(1.0 - r*r) * float(int(theta < 0.0) * 2 - 1);

	return vec2(r, i);
}

void fft_radix2(int logTwo, int btid, int g_offset, bool is_inverse, float N)
{
    for(int i = 0; i < logTwo; i++)
    {
        for(int j = btid; j < btid + g_offset; j++)
        {
            uint even = index_map(uint(j), uint(i), uint(N));
            uint odd = even + (1u << uint(i));
    
            vec2 evenVal = vec2(real_cache[even], imag_cache[even]);
    
            uint q = twiddle_map(uint(j), uint(i), uint(logTwo), uint(N));

            vec2 e = cplx_mul(twiddle(float(q), is_inverse, N), vec2(real_cache[odd], imag_cache[odd]));
    
            vec2 calculatedEven = evenVal + e;
            vec2 calculatedOdd = evenVal - e;

            real_cache[even] = calculatedEven.x;
            imag_cache[even] = calculatedEven.y;

            real_cache[odd] = calculatedOdd.x;
            imag_cache[odd] = calculatedOdd.y;
		}
        sync();
    }
}

void load_stage0(int btid, int g_offset, int scanline)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {
        int j = int(rev_bits(i) >> clz_width);
        
		pixel_buffer_real[i - btid * 2] = imageLoad(inputImage, ivec2(j, scanline));

		pixel_buffer_imag[i - btid * 2] = vec4(0.0);
    }
}

void store_stage0(int btid, int g_offset, int scanline)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {   
		ivec2 idx = ivec2(i, scanline);

		imageStore(realPart, idx, pixel_buffer_real[i - btid * 2]);
		
		imageStore(imagPart, idx, pixel_buffer_imag[i - btid * 2]);
    }
}

void load_stage1_2(int btid, int g_offset, int scanline)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {
        int j = int(rev_bits(i) >> clz_height);

		pixel_buffer_real[i - btid * 2] = imageLoad(realPart, ivec2(scanline, j));

		pixel_buffer_imag[i - btid * 2] = imageLoad(imagPart, ivec2(scanline, j));

    }
}

void store_stage1_2(int btid, int g_offset, int scanline, float N)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {        
		ivec2 idx = ivec2(scanline, i);

		vec4 colr = pixel_buffer_real[i - btid * 2] * N;
		vec4 coli = pixel_buffer_imag[i - btid * 2] * N;

		imageStore(realPart, idx, colr);
		imageStore(imagPart, idx, coli);
	
    }
}

void load_stage3(int btid, int g_offset, int scanline)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {
        int j = int(rev_bits(i) >> clz_width);

		pixel_buffer_real[i - btid * 2] = imageLoad(realPart, ivec2(j, scanline));

		pixel_buffer_imag[i - btid * 2] = imageLoad(imagPart, ivec2(j, scanline));
    }
}

void store_stage3(int btid, int g_offset, int scanline, float N)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {        
		if(i >= input_width) return;

		vec4 col = pixel_buffer_real[i - btid * 2] * N;
			
		imageStore(inputImage, ivec2(i, scanline), col);
    }
}

void load_into_cache(int btid, int g_offset, int channel)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {
		real_cache[i] = pixel_buffer_real[i - btid * 2][channel];
		imag_cache[i] = pixel_buffer_imag[i - btid * 2][channel];
	}
}

void load_from_cache(int btid, int g_offset, int channel)
{
	for(int i = btid * 2; i < btid * 2 + g_offset * 2; i++)
    {
		pixel_buffer_real[i - btid * 2][channel] = real_cache[i];
		pixel_buffer_imag[i - btid * 2][channel] = imag_cache[i];
	}
}

void main()
{
	switch(stage)
	{
		case 0:
		{
			int N = output_width;
			int g_offset = N / 2 / WORKGROUP_SIZE_X;
			int btid = int(g_offset * gl_LocalInvocationID.x);
			
			load_stage0(btid, g_offset, int(gl_WorkGroupID.x));
			sync();

			for(int channel = 0; channel < no_of_channels; channel++)
			{
				load_into_cache(btid, g_offset, channel);
				sync();
				
				fft_radix2(logtwo_width, btid, g_offset, false, N);
				sync();

				load_from_cache(btid, g_offset, channel);
			}
			
			store_stage0(btid, g_offset, int(gl_WorkGroupID.x));

			sync();

			return;
		}
		case 1:
		case 2:
		{
			int N = output_height;
			int g_offset = N / 2 / WORKGROUP_SIZE_X;
			int btid = int(g_offset * gl_LocalInvocationID.x);
			float divisor = (stage == 2) ? 1.0 / float(N) : 1.0;
			bool is_inverse = stage == 2;
			
			load_stage1_2(btid, g_offset, int(gl_WorkGroupID.x));
			sync();
			
			for(int channel = 0; channel < no_of_channels; channel++)
			{
				load_into_cache(btid, g_offset, channel);
				sync();

				fft_radix2(logtwo_height, btid, g_offset, is_inverse, N);
				sync();
				
				load_from_cache(btid, g_offset, channel);
			}

			sync();

			store_stage1_2(btid, g_offset, int(gl_WorkGroupID.x), divisor);

			return;
		}
		case 3:
		{
			int N = output_width;
			int g_offset = N / 2 / WORKGROUP_SIZE_X;
			int btid = int(g_offset * gl_LocalInvocationID.x);
			float divisor = 1.0 / float(N);

			load_stage3(btid, g_offset, int(gl_WorkGroupID.x));
			sync();

			for(int channel = 0; channel < no_of_channels; channel++)
			{
				load_into_cache(btid, g_offset, channel);
				sync();

				fft_radix2(logtwo_width, btid, g_offset, true, N);
				sync();
			
				load_from_cache(btid, g_offset, channel);
			}

			sync();

			store_stage3(btid, g_offset, int(gl_WorkGroupID.x), divisor);
		}
	}
}
