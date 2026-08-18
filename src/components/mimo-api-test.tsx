import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation } from "@tanstack/react-query";
import { api } from "@/lib/api";
import type { ApiProviderTestPayload, ApiProviderListPayload } from "@/types";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ButtonBusyContent } from "@/components/ui/button-busy-content";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { toast } from "@/hooks/use-toast";
import { useBusyAction } from "@/hooks/use-busy-action";
import { CheckCircle, XCircle, Zap, Save, Star } from "lucide-react";

// MiMo (Token Plan) is responses-native; its wire protocol is "responses".
const MIMO_BASE_URL = "https://token-plan-cn.xiaomimimo.com/v1";

interface MimoApiTestProps {
  className?: string;
}

export function MimoApiTest({ className }: MimoApiTestProps) {
  const { t } = useTranslation();
  const [apiKey, setApiKey] = useState("");
  const [testResult, setTestResult] = useState<ApiProviderTestPayload | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [store, setStore] = useState<ApiProviderListPayload | null>(null);

  const testMimoAction = useBusyAction({ minVisibleMs: 600 });
  const testResponsesAction = useBusyAction({ minVisibleMs: 600 });
  const loadModelsAction = useBusyAction({ minVisibleMs: 400 });
  const saveAction = useBusyAction({ minVisibleMs: 400 });
  const activateAction = useBusyAction({ minVisibleMs: 400 });

  const testMimoMutation = useMutation({
    mutationFn: async () => {
      const key = apiKey || undefined;
      const result = await api.testProviderSupport("mimo", undefined, key);
      return result;
    },
    onSuccess: (result) => {
      setTestResult(result.data);
      toast({
        title: t("mimoApiTest.success"),
        description: result.message,
      });
    },
    onError: (error) => {
      toast({
        title: t("mimoApiTest.error"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const testResponsesMutation = useMutation({
    mutationFn: async () => {
      const key = apiKey || undefined;
      const result = await api.testProtocolSupport("mimo", undefined, "responses", key);
      return result;
    },
    onSuccess: (result) => {
      const data = result.data;
      setTestResult((prev) => {
        if (!prev) return prev;
        const updated = { ...prev };
        const existingIndex = updated.protocolTests.findIndex(
          (t) => t.protocol === "responses"
        );
        if (existingIndex >= 0) {
          updated.protocolTests[existingIndex] = data;
        } else {
          updated.protocolTests.push(data);
        }
        updated.supportsResponses = data.supported;
        return updated;
      });
      toast({
        title: t("mimoApiTest.responsesTestSuccess"),
        description: result.message,
      });
    },
    onError: (error) => {
      toast({
        title: t("mimoApiTest.responsesTestError"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const loadModelsMutation = useMutation({
    mutationFn: async () => {
      const key = apiKey || undefined;
      const result = await api.getAvailableModels("mimo", undefined, key);
      return result;
    },
    onSuccess: (result) => {
      setModels(result.data);
      if (result.data.length > 0 && !selectedModel) {
        setSelectedModel(result.data[0]);
      }
      toast({
        title: t("mimoApiTest.modelsLoaded"),
        description: `${result.data.length} ${t("mimoApiTest.modelsFound")}`,
      });
    },
    onError: (error) => {
      toast({
        title: t("mimoApiTest.modelsError"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const key = apiKey || undefined;
      const result = await api.upsertApiProvider({
        providerType: "mimo",
        name: "MiMo (Token Plan, responses)",
        baseUrl: MIMO_BASE_URL,
        apiKey: key ?? null,
        supportsResponses: testResult?.supportsResponses ?? false,
        supportsChatCompletions: testResult?.supportsChatCompletions ?? false,
        modelList: models,
        defaultModel: selectedModel || models[0] || null,
      });
      return result;
    },
    onSuccess: (result) => {
      setStore(result.data);
      toast({
        title: t("mimoApiTest.saved"),
        description: t("mimoApiTest.savedDesc"),
      });
    },
    onError: (error) => {
      toast({
        title: t("mimoApiTest.saveError"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const activateMutation = useMutation({
    mutationFn: async () => {
      const result = await api.setActiveApiProvider("mimo", selectedModel || models[0] || undefined);
      return result;
    },
    onSuccess: (result) => {
      setStore(result.data);
      toast({
        title: t("mimoApiTest.activated"),
        description: t("mimoApiTest.activatedDesc"),
      });
    },
    onError: (error) => {
      toast({
        title: t("mimoApiTest.activateError"),
        description: error instanceof Error ? error.message : String(error),
        variant: "destructive",
      });
    },
  });

  const getStatusIcon = (supported: boolean) => {
    return supported ? (
      <CheckCircle className="h-4 w-4 text-green-500" />
    ) : (
      <XCircle className="h-4 w-4 text-red-500" />
    );
  };

  const getStatusBadge = (supported: boolean, label: string) => {
    return supported ? (
      <Badge variant="default" className="bg-green-500">
        {label}
      </Badge>
    ) : (
      <Badge variant="destructive">{label}</Badge>
    );
  };

  return (
    <Card className={cn("w-full", className)}>
      <CardHeader>
        <div className="flex items-center gap-2">
          <Zap className="h-5 w-5 text-blue-500" />
          <CardTitle>{t("mimoApiTest.title")}</CardTitle>
        </div>
        <CardDescription>{t("mimoApiTest.description")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label>{t("mimoApiTest.apiKey")}</Label>
          <Input
            type="password"
            placeholder={t("mimoApiTest.apiKeyPlaceholder")}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            {t("mimoApiTest.baseUrlHint")} {MIMO_BASE_URL}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          <Button
            onClick={() => testMimoAction.run(() => testMimoMutation.mutateAsync())}
            disabled={testMimoAction.busy}
          >
            <ButtonBusyContent
              busy={testMimoAction.busy}
              idleLabel={t("mimoApiTest.testMimo")}
            />
          </Button>

          <Button
            variant="outline"
            onClick={() => testResponsesAction.run(() => testResponsesMutation.mutateAsync())}
            disabled={testResponsesAction.busy}
          >
            <ButtonBusyContent
              busy={testResponsesAction.busy}
              idleLabel={t("mimoApiTest.testResponses")}
            />
          </Button>

          <Button
            variant="outline"
            onClick={() => loadModelsAction.run(() => loadModelsMutation.mutateAsync())}
            disabled={loadModelsAction.busy}
          >
            <ButtonBusyContent
              busy={loadModelsAction.busy}
              idleLabel={t("mimoApiTest.loadModels")}
            />
          </Button>
        </div>

        {testResult && (
          <div className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="flex items-center gap-2 p-3 border rounded-lg">
                {getStatusIcon(testResult.reachable)}
                <div>
                  <div className="text-sm font-medium text-muted-foreground">
                    {t("mimoApiTest.reachable")}
                  </div>
                  <div className="font-medium">
                    {testResult.reachable ? t("mimoApiTest.yes") : t("mimoApiTest.no")}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2 p-3 border rounded-lg">
                {getStatusIcon(testResult.supportsResponses)}
                <div>
                  <div className="text-sm font-medium text-muted-foreground">
                    {t("mimoApiTest.responsesProtocol")}
                  </div>
                  <div className="font-medium">
                    {getStatusBadge(
                      testResult.supportsResponses,
                      testResult.supportsResponses ? t("mimoApiTest.supported") : t("mimoApiTest.unsupported")
                    )}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-2 p-3 border rounded-lg">
                {getStatusIcon(testResult.supportsChatCompletions)}
                <div>
                  <div className="text-sm font-medium text-muted-foreground">
                    {t("mimoApiTest.chatCompletions")}
                  </div>
                  <div className="font-medium">
                    {getStatusBadge(
                      testResult.supportsChatCompletions,
                      testResult.supportsChatCompletions ? t("mimoApiTest.supported") : t("mimoApiTest.unsupported")
                    )}
                  </div>
                </div>
              </div>
            </div>

            <div className="text-sm text-muted-foreground">{testResult.message}</div>

            {testResult.protocolTests.length > 0 && (
              <div className="space-y-2">
                <h4 className="font-medium">{t("mimoApiTest.protocolDetails")}</h4>
                <div className="space-y-2">
                  {testResult.protocolTests.map((result, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between p-3 border rounded-lg"
                    >
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{result.protocol}</span>
                        <span className="text-sm text-muted-foreground">{result.endpoint}</span>
                      </div>
                      <div className="flex items-center gap-2">
                        {getStatusBadge(
                          result.supported,
                          result.supported ? t("mimoApiTest.supported") : t("mimoApiTest.unsupported")
                        )}
                        {result.statusCode && <Badge variant="outline">{result.statusCode}</Badge>}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {models.length > 0 && (
          <div className="space-y-2">
            <Label>{t("mimoApiTest.defaultModel")}</Label>
            <select
              className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
            >
              {models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </div>
        )}

        <div className="flex flex-wrap gap-2 pt-2">
          <Button
            variant="default"
            onClick={() => saveAction.run(() => saveMutation.mutateAsync())}
            disabled={saveAction.busy || !testResult?.supportsResponses}
          >
            <Save className="h-4 w-4 mr-1" />
            <ButtonBusyContent
              busy={saveAction.busy}
              idleLabel={t("mimoApiTest.saveProvider")}
            />
          </Button>

          <Button
            variant="outline"
            onClick={() => activateAction.run(() => activateMutation.mutateAsync())}
            disabled={activateAction.busy || !testResult?.supportsResponses}
          >
            <Star className="h-4 w-4 mr-1" />
            <ButtonBusyContent
              busy={activateAction.busy}
              idleLabel={t("mimoApiTest.setActive")}
            />
          </Button>
        </div>

        {store && (
          <div className="text-xs text-muted-foreground pt-1">
            {t("mimoApiTest.activeLabel")}:{" "}
            <span className="font-medium text-foreground">
              {store.activeProvider ?? t("mimoApiTest.none")}
            </span>
            {store.activeModel ? ` (${store.activeModel})` : ""}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
