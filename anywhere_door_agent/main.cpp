#include <Windows.h>
#include "service.h"

SERVICE_STATUS_HANDLE serviceStatusHandle = 0;
SERVICE_STATUS serviceStatus = {};
HANDLE stopEvent = nullptr;
wchar_t serviceName[] = L"AnywhereDoorAgent";

void UpdateServiceStatus(DWORD currentState, DWORD win32ExitCode, DWORD waitHint)
{
    static DWORD checkpoint = 1;

    serviceStatus.dwServiceType = SERVICE_WIN32_OWN_PROCESS;
    serviceStatus.dwCurrentState = currentState;
    serviceStatus.dwWin32ExitCode = win32ExitCode;
    serviceStatus.dwWaitHint = waitHint;

    if (currentState == SERVICE_START_PENDING)
    {
        serviceStatus.dwControlsAccepted = 0;
    }
    else
    {
        serviceStatus.dwControlsAccepted = SERVICE_ACCEPT_STOP | SERVICE_ACCEPT_SHUTDOWN;
    }

    if (currentState == SERVICE_RUNNING || currentState == SERVICE_STOPPED)
    {
        serviceStatus.dwCheckPoint = 0;
    }
    else
    {
        serviceStatus.dwCheckPoint = checkpoint++;
    }

    SetServiceStatus(serviceStatusHandle, &serviceStatus);
}

void WINAPI ServiceControlHandler(DWORD controlCode)
{
    switch (controlCode)
    {
    case SERVICE_CONTROL_STOP:
    case SERVICE_CONTROL_SHUTDOWN:
        if (serviceStatus.dwCurrentState != SERVICE_RUNNING)
        {
            return;
        }

        UpdateServiceStatus(SERVICE_STOP_PENDING, NO_ERROR, 3000);
        SetEvent(stopEvent);
        return;
    default:
        return;
    }
}

void WINAPI ServiceMain(DWORD argc, LPWSTR* argv)
{
    (void)argc;
    (void)argv;

    serviceStatusHandle = RegisterServiceCtrlHandlerW(serviceName, ServiceControlHandler);
    if (serviceStatusHandle == 0)
    {
        return;
    }

    UpdateServiceStatus(SERVICE_START_PENDING, NO_ERROR, 3000);

    stopEvent = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    if (stopEvent == nullptr)
    {
        UpdateServiceStatus(SERVICE_STOPPED, GetLastError(), 0);
        return;
    }

    // Simulate some initialization work
    startOrchestrator();
    Sleep(2000);


    UpdateServiceStatus(SERVICE_RUNNING, NO_ERROR, 0);

    WaitForSingleObject(stopEvent, INFINITE);

    CloseHandle(stopEvent);
    stopEvent = nullptr;

    UpdateServiceStatus(SERVICE_STOPPED, NO_ERROR, 0);
}

int main()
{
    SERVICE_TABLE_ENTRYW serviceTable[] = {
        { serviceName, ServiceMain },
        { nullptr, nullptr }
    };

    if (!StartServiceCtrlDispatcherW(serviceTable))
    {
        const DWORD error = GetLastError();
        if (error == ERROR_FAILED_SERVICE_CONTROLLER_CONNECT)
        {
            return 0;
        }

        return static_cast<int>(error);
    }

    return 0;
}