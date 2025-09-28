// Finds all occurrences of a given string in the program memory
// @category Search

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryAccessException;

public class FindDllImportExport extends GhidraScript {

    @Override
    public void run() throws Exception {
        // The string you want to search
        String target = askString("Search String", "Enter string to find:");

        Memory mem = currentProgram.getMemory();
        byte[] targetBytes = target.getBytes("UTF-8");
        println("Searching for " + target);
        // Iterate over all memory blocks
        for (var block : mem.getBlocks()) {
            Address start = block.getStart();
            Address end   = block.getEnd();
            long size = block.getSize();

            byte[] buffer = new byte[(int) size];
            try {
                mem.getBytes(start, buffer);
            } catch (MemoryAccessException e) {
                println("Could not read memory block: " + block.getName());
                continue;
            }

            // Naive scan
            for (int i = 0; i <= buffer.length - targetBytes.length; i++) {
                boolean match = true;
                for (int j = 0; j < targetBytes.length; j++) {
                    if (buffer[i + j] != targetBytes[j]) {
                        match = false;
                        break;
                    }
                }
                if (match) {
                    Address foundAddr = start.add(i);
                    println("Found \"" + target + "\" at " + foundAddr);
                }
            }
        }
    }
}

