#include <Windows.h>
#include <iostream>
#include "service.h"
#include <fstream>

namespace
{
    std::string getProbeFilePath()
    {
        char exePath[MAX_PATH] = {};
        if (GetModuleFileNameA(nullptr, exePath, MAX_PATH) == 0)
        {
            return "sample.txt";
        }

        std::string path(exePath);
        const size_t lastSlash = path.find_last_of("\\/");
        if (lastSlash != std::string::npos)
        {
            path.resize(lastSlash + 1);
        }
        else
        {
            path.clear();
        }

        path += "sample.txt";
        return path;
    }
}

void startOrchestrator()
{
    // Simulate starting the orchestrator
    const std::string probeFilePath = getProbeFilePath();
    std::ofstream outputFile(probeFilePath, std::ios::out | std::ios::trunc);
    if (outputFile.is_open())
    {
        outputFile << "Orchestrator started successfully! and written to file" << std::endl;
        outputFile << "Probe file path: " << probeFilePath << std::endl;
        outputFile.close();
    }
    Sleep(1000); // Simulate some delay
}