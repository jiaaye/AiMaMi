import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type {
  ApiProviderType,
  ApiProviderTestPayload,
  ApiProtocolTestPayload,
} from "@/types";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ButtonBusyContent } from "@/components/ui/button-busy-content";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { toast } from "@/hooks/use-toast";
import { useBusyAction } from "@/hooks/use-busy-action";

interface ApiProviderTestProps {
  className?: string;
}

export function ApiProviderTest({ className }: ApiProviderTestProps) {
  const { t } = useTranslation();
  const [selectedProvider, setSelectedProvider] = useState<ApiProviderType>("mimo");
  const [customUrl, setCustomUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [testResult, setTestResult] = useState<ApiProviderTestPayload | null>(null);
  const [protocolTestResults, setProtocolTestResults] = useState<ApiProtocolTestPayload[]>([]);
  
  const testProviderAction = useBusyAction({ minVisibleMs: 600 });
  const testProtocolAction = useBusyAction({ minVisibleMs: 600 });

  const testProviderMutation = useMutation({
    mutationFn: async () => {
      const url = selectedProvider === "custom" ? customUrl : undefined;
      const key = apiKey || undefined;
      const result = await api.testProviderSupport(selectedProvider, url, key);
      return result;
    },
    onSuccess: (result) => {
      setTestResult(result);
      setProtocolTestResults(result.protocolTests);
      toast({
        title: t("apiProviderTest.success"),
        description: result.message,
      });
    },
    onError: (error) => {
      toast({
        title: t("apiProviderTest.error"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const testProtocolMutation = useMutation({
    mutationFn: async (protocol: string) => {
      const url = selectedProvider === "custom" ? customUrl : undefined;
      const key = apiKey || undefined;
      const result = await api.testProtocolSupport(selectedProvider, url, protocol, key);
      return result;
    },
    onSuccess: (result) => {
      setProtocolTestResults((prev) => {
        const existing = prev.findIndex((r) => r.protocol === result.protocol);
        if (existing >= 0) {
          const updated = [...prev];
          updated[existing] = result;
          return updated;
        }
        return [...prev, result];
      });
      toast({
        title: t("apiProtocolTest.success"),
        description: result.message,
      });
    },
    onError: (error) => {
      toast({
        title: t("apiProtocolTest.error"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const getProviderName = (provider: ApiProviderType) => {
    switch (provider) {
      case "openai":
        return "OpenAI";
      case "deepseek":
        return "DeepSeek";
      case "mimo":
        return "MiMo";
      case "custom":
        return "Custom";
      default:
        return provider;
    }
  };

  const getStatusBadge = (supported: boolean) => {
    return supported ? (
      <Badge variant="default" className="bg-green-500">
        {t("apiProviderTest.supported")}
      </Badge>
    ) : (
      <Badge variant="destructive">
        {t("apiProviderTest.unsupported")}
      </Badge>
    );
  };

  return (
    <Card className={cn("w-full", className)}>
      <CardHeader>
        <CardTitle>{t("apiProviderTest.title")}</CardTitle>
        <CardDescription>
          {t("apiProviderTest.description")}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label>{t("apiProviderTest.provider")}</Label>
            <Select
              value={selectedProvider}
              onValueChange={(value) => setSelectedProvider(value as ApiProviderType)}
            >
              <SelectTrigger>
                <SelectValue placeholder={t("apiProviderTest.selectProvider")} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="openai">OpenAI</SelectItem>
                <SelectItem value="deepseek">DeepSeek</SelectItem>
                <SelectItem value="mimo">MiMo</SelectItem>
                <SelectItem value="custom">{t("apiProviderTest.custom")}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {selectedProvider === "custom" && (
            <div className="space-y-2">
              <Label>{t("apiProviderTest.customUrl")}</Label>
              <Input
                placeholder="https://api.example.com"
                value={customUrl}
                onChange={(e) => setCustomUrl(e.target.value)}
              />
            </div>
          )}

          <div className="space-y-2">
            <Label>{t("apiProviderTest.apiKey")}</Label>
            <Input
              type="password"
              placeholder={t("apiProviderTest.apiKeyPlaceholder")}
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
          </div>
        </div>

        <div className="flex gap-2">
          <Button
            onClick={() => testProviderAction.wrap(() => testProviderMutation.mutateAsync())}
            disabled={testProviderAction.isBusy}
          >
            <ButtonBusyContent isBusy={testProviderAction.isBusy}>
              {t("apiProviderTest.testProvider")}
            </ButtonBusyContent>
          </Button>

          <Button
            variant="outline"
            onClick={() => testProtocolAction.wrap(() => testProtocolMutation.mutateAsync("responses"))}
            disabled={testProtocolAction.isBusy}
          >
            <ButtonBusyContent isBusy={testProtocolAction.isBusy}>
              {t("apiProviderTest.testResponses")}
            </ButtonBusyContent>
          </Button>

          <Button
            variant="outline"
            onClick={() => testProtocolAction.wrap(() => testProtocolMutation.mutateAsync("chat_completions"))}
            disabled={testProtocolAction.isBusy}
          >
            <ButtonBusyContent isBusy={testProtocolAction.isBusy}>
              {t("apiProviderTest.testChatCompletions")}
            </ButtonBusyContent>
          </Button>
        </div>

        {testResult && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div className="space-y-1">
                <div className="text-sm font-medium text-muted-foreground">
                  {t("apiProviderTest.provider")}
                </div>
                <div className="font-medium">{getProviderName(testResult.provider)}</div>
              </div>
              
              <div className="space-y-1">
                <div className="text-sm font-medium text-muted-foreground">
                  {t("apiProviderTest.reachable")}
                </div>
                <div>
                  {testResult.reachable ? (
                    <Badge variant="default" className="bg-green-500">
                      {t("apiProviderTest.yes")}
                    </Badge>
                  ) : (
                    <Badge variant="destructive">
                      {t("apiProviderTest.no")}
                    </Badge>
                  )}
                </div>
              </div>

              <div className="space-y-1">
                <div className="text-sm font-medium text-muted-foreground">
                  {t("apiProviderTest.responses")}
                </div>
                <div>{getStatusBadge(testResult.supportsResponses)}</div>
              </div>

              <div className="space-y-1">
                <div className="text-sm font-medium text-muted-foreground">
                  {t("apiProviderTest.chatCompletions")}
                </div>
                <div>{getStatusBadge(testResult.supportsChatCompletions)}</div>
              </div>
            </div>

            <div className="text-sm text-muted-foreground">
              {testResult.message}
            </div>
          </div>
        )}

        {protocolTestResults.length > 0 && (
          <div className="space-y-4">
            <h4 className="font-medium">{t("apiProviderTest.protocolDetails")}</h4>
            <div className="space-y-2">
              {protocolTestResults.map((result, index) => (
                <div
                  key={index}
                  className="flex items-center justify-between p-3 border rounded-lg"
                >
                  <div className="flex items-center gap-2">
                    <span className="font-medium">{result.protocol}</span>
                    <span className="text-sm text-muted-foreground">
                      {result.endpoint}
                    </span>
                  </div>
                  <div className="flex items-center gap-2">
                    {getStatusBadge(result.supported)}
                    {result.statusCode && (
                      <Badge variant="outline">
                        {result.statusCode}
                      </Badge>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
