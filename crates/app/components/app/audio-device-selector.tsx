import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from "@/components/ui/select";
import { Label } from "../ui/label";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { invoke_typed } from "@/lib/utils";

enum DeviceType {
    Input = "Input",
    Output = "Output",
}

interface DeviceInfo {
    name: string;
    device_type: DeviceType;
    active: boolean;
    default: boolean;
}

function AudioDeviceSelector() {
    const [devices, setDevices] = useState<Array<DeviceInfo>>(new Array<DeviceInfo>());
    const [inputDevice, setInputDevice] = useState<string | undefined>(undefined);
    const [outputDevice, setOutputDevice] = useState<string | undefined>(undefined);
    const [isRunning, setIsRunning] = useState<boolean>(false);

    async function refeshIsRunning() {
        try {
            let res = await invoke_typed<boolean>("is_running");
            setIsRunning(res);
            console.log(res);
        } catch (err) {
            console.error("Error fetching devices:", err);
        }
    }

    async function getDevices() {
        try {
            let res = await invoke_typed<Array<DeviceInfo>>("get_devices");
            console.log(res);
            setDevices(res || new Array<DeviceInfo>());

            let input = res.find((d) => d.device_type === DeviceType.Input && d.active);
            setInputDevice(input?.name);

            let output = res.find((d) => d.device_type === DeviceType.Output && d.active);
            setOutputDevice(output?.name);
        } catch (err) {
            console.error("Error fetching devices:", err);
        }
    }

    useEffect(() => {
        getDevices();

        const interval = setInterval(refeshIsRunning, 200);
        return () => clearInterval(interval);
    }, []);

    async function handleOutputSelect(deviceId: string) {
        let device = devices.find((d) => d.name === deviceId);
        if (!device) {
            console.error("Device not found:", deviceId);
            return;
        }

        try {
            let res = await invoke("set_device", { deviceType: device.device_type, deviceName: deviceId });
            console.log(res);
        } catch (err) {
            console.error("Failed to set output device:", err);
        }
    }

    async function start() {
        try {
            let res = await invoke("start");
            console.log(res);
        } catch (err) {
            console.error("Failed to start:", err);
        }
    }

    async function stop() {
        try {
            let res = await invoke("stop");
            console.log(res);
        } catch (err) {
            console.error("Failed to stop:", err);
        }
    }

    return (
        <div className="p-4 flex flex-col gap-2">
            {isRunning ? <Badge variant="outline">Running</Badge> : <Badge variant="destructive">Stopped</Badge>}

            <Label htmlFor="microphone">Microphone</Label>
            <Select onValueChange={handleOutputSelect} value={inputDevice}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Microphone" />
                </SelectTrigger>
                <SelectContent position="item-aligned" align="center">
                    {devices
                        .filter((d) => d.device_type === DeviceType.Input)
                        .map((device) => (
                            <SelectItem key={device.name} value={device.name}>
                                {device.name || "Unknown Microphone"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>

            <Label htmlFor="speakers">Speakers</Label>
            <Select onValueChange={handleOutputSelect} value={outputDevice}>
                <SelectTrigger className="w-full">
                    <SelectValue placeholder="Select Output" />
                </SelectTrigger>
                <SelectContent position="item-aligned" align="center">
                    {devices
                        .filter((d) => d.device_type === DeviceType.Output)
                        .map((device) => (
                            <SelectItem key={device.name} value={device.name}>
                                {device.name || "Unknown output"}
                            </SelectItem>
                        ))}
                </SelectContent>
            </Select>

            <Button onClick={isRunning ? stop : start} variant={isRunning ? "destructive" : "default"} className="w-full">
                {isRunning ? "Stop" : "Start"}
            </Button>
            <Button onClick={getDevices}>Refresh Devices</Button>
        </div>
    );
}

export default AudioDeviceSelector;
