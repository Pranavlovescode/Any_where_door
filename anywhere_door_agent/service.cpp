#include "service.h"

#include <chrono>
#include <csignal>
#include <cstdlib>
#include <fstream>
#include <iostream>
#include <string>
#include <thread>

namespace
{
    volatile std::sig_atomic_t gStopRequested = 0;

    void handleSignal(int)
    {
        gStopRequested = 1;
    }

    std::string getProbeFilePath()
    {
        const char* customProbePath = std::getenv("ANYWHERE_DOOR_PROBE_FILE");
        if (customProbePath != nullptr && customProbePath[0] != '\0')
        {
            return customProbePath;
        }

        return "output/sample.txt";
    }

    std::string getHeartbeatMessage()
    {
        const auto now = std::chrono::system_clock::now();
        const auto epochSeconds = std::chrono::duration_cast<std::chrono::seconds>(
            now.time_since_epoch()).count();

        return "Orchestrator heartbeat epoch=" + std::to_string(epochSeconds);
    }
}

void startOrchestrator()
{
    const std::string probeFilePath = getProbeFilePath();
    std::ofstream outputFile(probeFilePath, std::ios::out | std::ios::app);
    if (outputFile.is_open())
    {
        outputFile << getHeartbeatMessage() << std::endl;
        outputFile << "Probe file path: " << probeFilePath << std::endl;
        outputFile.close();
        return;
    }

    std::cerr << "Unable to open probe file at: " << probeFilePath << std::endl;
}

int runService()
{
    gStopRequested = 0;

    std::signal(SIGINT, handleSignal);
#ifdef SIGTERM
    std::signal(SIGTERM, handleSignal);
#endif

    std::cout << "Anywhere Door agent service started. Press Ctrl+C to stop." << std::endl;

    while (gStopRequested == 0)
    {
        startOrchestrator();
        std::this_thread::sleep_for(std::chrono::seconds(2));
    }

    std::cout << "Anywhere Door agent service stopped." << std::endl;
    return 0;
}