#import <Foundation/Foundation.h>
#import <Metal/Metal.h>
#import "MetalPoseidon.h"

int main(int argc, const char * argv[]) {
    @autoreleasepool {
        id<MTLDevice> device = MTLCreateSystemDefaultDevice();

        // Create the custom object used to encapsulate the Metal code.
        // Initializes objects to communicate with the GPU.
        MetalPoseidon* adder = [[MetalPoseidon alloc] initWithDevice:device];
        
        // Create buffers to hold data
        [adder prepareData];
        
        // Send a command to the GPU to perform the calculation.
        [adder sendComputeCommand];

        NSLog(@"Execution finished");
    }
    return 0;
}
