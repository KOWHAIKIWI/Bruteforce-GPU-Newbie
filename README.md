# OpenCL GPU Environment Setup and Testing

This document provides an overview of the tests executed to verify the OpenCL environment setup and ensure that the GPU (NVIDIA GeForce RTX 4090) is properly recognized and functional for OpenCL workloads.

---

## **Goal**
To ensure that a script or program can detect and activate the GPU for OpenCL-based computations.

---

## **Tests Executed**

### **1. Check OpenCL Platforms with Custom Program**
- **Purpose**: To verify that the OpenCL runtime and drivers are properly installed, and the GPU is detected.
- **Tool Used**: A custom OpenCL test program (`test_opencl.c`).
- **Result**: Successfully detected 1 OpenCL platform (`NVIDIA CUDA`) and the RTX 4090 GPU.

#### **Code Used**
```c
#define CL_TARGET_OPENCL_VERSION 120
#include <CL/cl.h>
#include <stdio.h>

int main() {
    cl_uint num_platforms;
    cl_int err = clGetPlatformIDs(0, NULL, &num_platforms);
    if (err != CL_SUCCESS) {
        printf("Error getting OpenCL platforms: %d\n", err);
        return 1;
    }
    printf("Number of OpenCL platforms: %u\n", num_platforms);

    cl_platform_id platforms[num_platforms];
    clGetPlatformIDs(num_platforms, platforms, NULL);

    for (cl_uint i = 0; i < num_platforms; i++) {
        char platform_name[128];
        clGetPlatformInfo(platforms[i], CL_PLATFORM_NAME, 128, platform_name, NULL);
        printf("Platform %u: %s\n", i, platform_name);
    }
    return 0;
}
```

#### **Compilation and Execution**
```bash
gcc -o test_opencl test_opencl.c -lOpenCL
./test_opencl
```

---

### **2. Run `clinfo -l`**
- **Purpose**: To list available OpenCL platforms and devices.
- **Command**:
  ```bash
  clinfo -l
  ```
- **Result**:
  ```
  Platform #0: NVIDIA CUDA
   `-- Device #0: NVIDIA GeForce RTX 4090
  ```

---

### **3. Debug `clinfo` Issues**
- **Purpose**: To diagnose why the full `clinfo` tool fails to display GPU details.
- **Command Used**:
  ```bash
  OCL_ICD_DEBUG=1 clinfo
  ```
- **Result**: Logs showed the library `libnvidia-opencl.so.1` was successfully opened, but `clinfo` still failed to display GPU properties.

---

### **4. Verify NVIDIA GPU Accessibility**
- **Purpose**: To confirm GPU functionality outside of OpenCL.
- **Tool Used**: `nvidia-smi`.
- **Command**:
  ```bash
  nvidia-smi
  ```
- **Result**: The GPU (RTX 4090) was successfully detected, and all processes were running correctly.

---

### **5. Validate OpenCL Runtime**
- **Purpose**: To manually check if the OpenCL library (`libnvidia-opencl.so.1`) is present and accessible.
- **Commands**:
  ```bash
  ls /usr/lib/x86_64-linux-gnu/libnvidia-opencl.so.1
  ldd /usr/lib/x86_64-linux-gnu/libnvidia-opencl.so.1
  ```
- **Result**: The library was found and all dependencies were resolved.

---

### **6. Write Extended OpenCL Query Program**
- **Purpose**: To replace `clinfo` by querying detailed OpenCL properties programmatically.
- **Code**:
```c
#define CL_TARGET_OPENCL_VERSION 120
#include <CL/cl.h>
#include <stdio.h>
#include <stdlib.h>

void print_device_info(cl_device_id device) {
    char device_name[256];
    cl_uint compute_units;
    cl_ulong global_mem_size;
    cl_ulong max_alloc_size;
    size_t max_work_group_size;

    clGetDeviceInfo(device, CL_DEVICE_NAME, sizeof(device_name), device_name, NULL);
    clGetDeviceInfo(device, CL_DEVICE_MAX_COMPUTE_UNITS, sizeof(compute_units), &compute_units, NULL);
    clGetDeviceInfo(device, CL_DEVICE_GLOBAL_MEM_SIZE, sizeof(global_mem_size), &global_mem_size, NULL);
    clGetDeviceInfo(device, CL_DEVICE_MAX_MEM_ALLOC_SIZE, sizeof(max_alloc_size), &max_alloc_size, NULL);
    clGetDeviceInfo(device, CL_DEVICE_MAX_WORK_GROUP_SIZE, sizeof(max_work_group_size), &max_work_group_size, NULL);

    printf("    Device Name: %s\n", device_name);
    printf("    Compute Units: %u\n", compute_units);
    printf("    Global Memory Size: %lu MB\n", global_mem_size / (1024 * 1024));
    printf("    Max Memory Allocation: %lu MB\n", max_alloc_size / (1024 * 1024));
    printf("    Max Work Group Size: %zu\n", max_work_group_size);
}

int main() {
    cl_uint num_platforms;
    cl_int err = clGetPlatformIDs(0, NULL, &num_platforms);
    if (err != CL_SUCCESS) {
        printf("Error getting OpenCL platforms: %d\n", err);
        return 1;
    }
    printf("Number of OpenCL platforms: %u\n", num_platforms);

    cl_platform_id platforms[num_platforms];
    clGetPlatformIDs(num_platforms, platforms, NULL);

    for (cl_uint i = 0; i < num_platforms; i++) {
        char platform_name[256];
        clGetPlatformInfo(platforms[i], CL_PLATFORM_NAME, sizeof(platform_name), platform_name, NULL);
        printf("Platform %u: %s\n", i, platform_name);

        cl_uint num_devices;
        clGetDeviceIDs(platforms[i], CL_DEVICE_TYPE_ALL, 0, NULL, &num_devices);
        printf("  Number of devices: %u\n", num_devices);

        cl_device_id devices[num_devices];
        clGetDeviceIDs(platforms[i], CL_DEVICE_TYPE_ALL, num_devices, devices, NULL);

        for (cl_uint j = 0; j < num_devices; j++) {
            printf("  Device %u:\n", j);
            print_device_info(devices[j]);
        }
    }

    return 0;
}
```

#### **Compilation and Execution**
```bash
gcc -o opencl_query opencl_query.c -lOpenCL
./opencl_query
```

---

### **7. Execute OpenCL Kernel Workload**
- **Purpose**: To verify GPU functionality by running an OpenCL workload.
- **Code Used**:
```c
#define CL_TARGET_OPENCL_VERSION 120
#include <CL/cl.h>
#include <stdio.h>
#include <stdlib.h>

const char *kernel_source = 
"__kernel void vector_add(__global const float *a, __global const float *b, __global float *c) {\n"
"    int id = get_global_id(0);\n"
"    c[id] = a[id] + b[id];\n"
"}\n";

int main() {
    const int elements = 1024;
    size_t bytes = elements * sizeof(float);

    float *h_a = (float *)malloc(bytes);
    float *h_b = (float *)malloc(bytes);
    float *h_c = (float *)malloc(bytes);

    for (int i = 0; i < elements; i++) {
        h_a[i] = i;
        h_b[i] = i * 2;
    }

    cl_platform_id platform;
    clGetPlatformIDs(1, &platform, NULL);

    cl_device_id device;
    clGetDeviceIDs(platform, CL_DEVICE_TYPE_GPU, 1, &device, NULL);

    cl_context context = clCreateContext(NULL, 1, &device, NULL, NULL, NULL);
    cl_command_queue queue = clCreateCommandQueue(context, device, 0, NULL);

    cl_mem d_a = clCreateBuffer(context, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, bytes, h_a, NULL);
    cl_mem d_b = clCreateBuffer(context, CL_MEM_READ_ONLY | CL_MEM_COPY_HOST_PTR, bytes, h_b, NULL);
    cl_mem d_c = clCreateBuffer(context, CL_MEM_WRITE_ONLY, bytes, NULL, NULL);

    cl_program program = clCreateProgramWithSource(context, 1, &kernel_source, NULL, NULL);
    clBuildProgram(program, 1, &device, NULL, NULL, NULL);

    cl_kernel kernel = clCreateKernel(program, "vector_add", NULL);

    clSetKernelArg(kernel, 0, sizeof(cl_mem), &d_a);
    clSetKernelArg(kernel, 1, sizeof(cl_mem), &d_b);
    clSetKernelArg(kernel, 2, sizeof(cl_mem), &d_c);

    size_t global_size = elements;
    clEnqueueNDRangeKernel(queue, kernel, 1, NULL, &global_size, NULL, 0, NULL, NULL);

    clEnqueueReadBuffer(queue, d_c, CL_TRUE, 0, bytes, h_c, 0, NULL, NULL);

    for (int i = 0; i < 10; i++) {
        printf("Result %d: %f\n", i, h_c[i]);
    }

    clReleaseMemObject(d_a);
    clReleaseMemObject(d_b);
    clReleaseMemObject(d_c);
    clReleaseKernel(kernel);
    clReleaseProgram(program);
    clReleaseCommandQueue(queue);
    clReleaseContext(context);

    free(h_a);
    free(h_b);
    free(h_c);

    return 0;
}
```

#### **Compilation and Execution**
```bash
gcc -o vector_add vector_add.c -lOpenCL
./vector_add
```

#### **Result**
The program output confirms the GPU is fully functional:
```
Result 0: 0.000000
Result 1: 3.000000
Result 2: 6.000000
Result 3: 9.000000
Result 4: 12.000000
Result 5: 15.000000
Result 6: 18.000000
Result 7: 21.000000
Result 8: 24.000000
Result 9: 27.000000
```

---

## **Key Findings**
- The OpenCL environment is correctly set up, and the GPU is fully functional for OpenCL workloads.
- The `clinfo` tool has limitations or bugs but is non-critical for your setup.

---

## **Recommendations**
- Use the extended OpenCL query program (`opencl_query.c`) for replacing `clinfo`.
- Alternatively, use `PyOpenCL` if Python-based diagnostics are preferred.
- For GPU-specific monitoring, rely on `nvidia-smi`.

---

## **Next Steps**
- Integrate the working OpenCL environment into your script to activate and utilize the GPU for OpenCL computations.
- Test your script with the extended OpenCL query program to ensure compatibility.

---