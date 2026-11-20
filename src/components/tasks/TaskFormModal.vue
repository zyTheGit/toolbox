<template>
  <n-modal
    :show="show"
    preset="card"
    :title="title"
    :style="{
      width: '640px',
      maxWidth: 'calc(100vw - 48px)',
      marginTop: '8vh',
      alignSelf: 'flex-start',
    }"
    :mask-closable="false"
    :bordered="false"
    @update:show="onShowChange"
  >
    <n-alert v-if="apiError" type="error" :bordered="false" class="modal-alert">
      {{ apiError }}
    </n-alert>

    <n-form label-placement="top" :show-feedback="true" :disabled="submitting">
      <TaskScheduleFields
        :form="form"
        :errors="errors"
        :repeat="repeat"
        :preview="preview"
        :preview-error="previewError"
        :previewing="previewing"
        :summary="summary"
        :cron-validation="cronValidation"
        :disabled="submitting"
      />

      <TaskPayloadFields :form="form" :errors="errors" :shell-options="shellOptions" />

      <n-form-item label="状态">
        <n-checkbox v-model:checked="form.enabled">
          {{ isEdit ? "启用任务" : "创建后立即启用" }}
        </n-checkbox>
      </n-form-item>
    </n-form>

    <template #footer>
      <div class="modal-foot">
        <n-button @click="close">取消</n-button>
        <n-button type="primary" :loading="submitting" @click="save">
          <template #icon>
            <AppIcon name="check" :size="16" />
          </template>
          {{ saveLabel }}
        </n-button>
      </div>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { NAlert, NButton, NCheckbox, NForm, NFormItem, NModal } from "naive-ui";
import type { Task } from "../../types";
import AppIcon from "../AppIcon.vue";
import TaskScheduleFields from "./TaskScheduleFields.vue";
import TaskPayloadFields from "./TaskPayloadFields.vue";
import { useTaskForm } from "./useTaskForm";

const props = defineProps<{
  show: boolean;
  task: Task | null;
  /** 复制来源：新建表单预填该任务的内容 */
  template?: Task | null;
  allowCommands: boolean;
}>();

const emit = defineEmits<{
  (e: "update:show", value: boolean): void;
  (e: "saved"): void;
}>();

const {
  form,
  errors,
  apiError,
  submitting,
  repeat,
  preview,
  previewError,
  previewing,
  isEdit,
  summary,
  cronValidation,
  shellOptions,
  save,
  close,
  onShowChange,
} = useTaskForm(props, emit);

const isClone = computed(() => !props.task && Boolean(props.template));
const title = computed(() => (isEdit.value ? "编辑任务" : isClone.value ? "复制任务" : "新建任务"));
const saveLabel = computed(() =>
  isEdit.value ? "保存修改" : isClone.value ? "创建副本" : "创建任务",
);
</script>

<style scoped>
.modal-alert {
  margin-bottom: 14px;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
